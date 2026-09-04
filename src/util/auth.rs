use std::{collections::HashMap, path::Path, process::exit, sync::Arc};

use actix_web::{Either, HttpResponse, HttpServer, http::header::ContentType, web::Query};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::ValueEnum;
use indoc::indoc;
use oo7::{Item, Keyring};
use openid::{Bearer, Client, Configurable, Provider, StandardClaims, Token, jwks};
use rustls_pki_types::pem::PemObject as _;
use serde::Deserialize;
use tokio::sync::mpsc::{Sender, channel};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::util::AuthEnvironment;

pub struct QBAuthData {
    pub environment: AuthEnvironment,
    pub realm: String,
    pub token: Token,
}

impl QBAuthData {
    pub async fn refresh_token(&mut self, oauth_client: &Client<QBSandboxProvider>) -> Result<()> {
        let old_token = Box::new(self.token.bearer.clone());
        self.token.bearer = oauth_client.refresh_token(&old_token, None).await?;
        // manually copy over the old id_token so that it isn't lost
        self.token.bearer.id_token = old_token.id_token;
        Ok(())
    }
}

pub async fn get_stored_profile_auth_token(
    keyring: &Keyring,
    profile: &str,
) -> Result<Option<QBAuthData>> {
    Ok(
        match get_stored_profile_auth_data(keyring, profile).await? {
            Some(item) => {
                let secret = item.secret().await?;
                let bearer = toml::from_slice::<Bearer>(secret.as_bytes())?;
                debug!("bearer: {:#?}", bearer);

                let mut attrs = item.attributes().await?;
                let realm_id = attrs.remove("realm").unwrap();
                let environment = attrs.remove("environment").unwrap();

                let token = Token::from(bearer);

                let auth_env = AuthEnvironment::from_str(environment.as_ref(), true)
                    .map_err(|e| anyhow::anyhow!(e))?;

                Some(QBAuthData {
                    environment: auth_env,
                    realm: realm_id.to_owned(),
                    token: token,
                })
            }
            None => None,
        },
    )
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct QBAuthRedirectData {
    pub(crate) code: String,
    pub(crate) state: String,
    #[serde(rename = "realmId")]
    pub(crate) realm_id: String,
}

#[derive(Clone)]
struct ExpectedAuthState(Arc<String>);

pub(crate) const SANDBOX_DISC_URL: &str =
    "https://developer.api.intuit.com/.well-known/openid_sandbox_configuration";
pub(crate) const PRODUCTION_DISC_URL: &str =
    "https://developer.api.intuit.com/.well-known/openid_configuration";

#[derive(Debug)]
pub(crate) struct QBSandboxProvider {
    pub(crate) config: openid::Config,
}

impl From<openid::Config> for QBSandboxProvider {
    fn from(config: openid::Config) -> Self {
        Self { config }
    }
}

pub async fn qb_provider_config(disc_url: &str) -> Result<openid::Config, anyhow::Error> {
    let client = reqwest::Client::new();
    let resp = client.get(disc_url).send().await?.error_for_status()?;
    let config: openid::Config = resp.json().await.unwrap();
    Ok(config)
}

impl Provider for QBSandboxProvider {
    fn auth_uri(&self) -> &url::Url {
        &self.config.authorization_endpoint
    }

    fn token_uri(&self) -> &url::Url {
        &self.config.token_endpoint
    }
}

impl Configurable for QBSandboxProvider {
    fn config(&self) -> &openid::Config {
        &self.config
    }
}

pub(crate) fn get_login_response_server(
    tx: Sender<QBAuthRedirectData>,
    shutdown_signal: CancellationToken,
    hostname: impl AsRef<str>,
    port: u16,
    expected_state: Arc<String>,
) -> Result<actix_web::dev::Server> {
    let cert_chain: Vec<rustls_pki_types::CertificateDer<'_>> =
        rustls_pki_types::CertificateDer::pem_file_iter(Path::new(
            "./dev/localhost-keys/localhost.crt",
        ))?
        .collect::<Result<_, _>>()
        .unwrap();

    let key = rustls_pki_types::PrivatePkcs8KeyDer::from_pem_file(Path::new(
        "./dev/localhost-keys/localhost.key",
    ))?;

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, rustls::pki_types::PrivateKeyDer::Pkcs8(key))?;

    let hostname = hostname.as_ref();
    info!("Waiting for login on {hostname}:{port}");
    Ok(HttpServer::new(move || {
        actix_web::App::new()
            .app_data(actix_web::web::Data::new(tx.clone()))
            .app_data(actix_web::web::Data::new(ExpectedAuthState(
                expected_state.clone(),
            )))
            .route(
                "/",
                actix_web::web::get().to(
                    |query: Query<QBAuthRedirectData>,
                     tx: actix_web::web::Data<Sender<QBAuthRedirectData>>,
                     expected_state: actix_web::web::Data<ExpectedAuthState>| async move {
                        if query.state != *expected_state.0 {
                            debug!(
                                "Received incorrect state. expected: {}, received: {}",
                                *expected_state.0, query.state
                            );
                            info!("Incorrect state received. Try logging in again.");
                            return Either::Right(HttpResponse::BadRequest().body(
                                "Incorrect state recieved. Not logged in. Please try again...",
                            ));
                        }
                        let auth_info = query.into_inner().clone();
                        tx.send(auth_info).await.unwrap();
                        let mut buf = String::new();
                        buf.push_str(indoc!(
                            "
                            <!DOCTYPE html>
                            <html>
                              <head>
                                <title>Logged In</title>
                              </head>
                              <body>
                                <p>Logged in. Please close this tab and return to your terminal.</p>
                              </body>
                            </html>
                            "
                        ));
                        Either::Left(
                            HttpResponse::Ok()
                                .insert_header(ContentType::html())
                                .body(buf),
                        )
                    },
                ),
            )
    })
    .workers(1)
    .shutdown_signal(shutdown_signal.cancelled_owned())
    .keep_alive(None)
    .bind_rustls_0_23(("127.0.0.1", port), tls_config.clone())?
    .bind_rustls_0_23(("::1", port), tls_config.clone())?
    //.bind(("127.0.0.1", port))
    .run())
}

const SCOPES: &str = concat!(
    "openid email profile address phone ",
    "com.intuit.quickbooks.accounting com.intuit.quickbooks.payment"
);

pub async fn get_qb_auth_data(
    oauth_client: &Client<QBSandboxProvider>,
    listen_host: impl AsRef<str>,
    listen_port: u16,
    realm: Option<impl AsRef<str>>,
) -> Result<QBAuthRedirectData> {
    let listen_host = listen_host.as_ref();
    let state_bytes: [u8; 8] = rand::random();
    let encoded_state = STANDARD.encode(state_bytes);

    let url = oauth_client.auth_url(&openid::Options {
        scope: Some(SCOPES.to_owned()),
        state: Some(encoded_state.clone()),
        ..openid::Options::default()
    });
    info!("auth url: {}", url);

    // create channel
    let (tx, mut rx) = channel(10);

    // spawn server task
    let server_shutdown_token = CancellationToken::new();
    let server = get_login_response_server(
        tx,
        server_shutdown_token.child_token(),
        &listen_host,
        listen_port,
        Arc::new(encoded_state),
    )?;
    let t1 = tokio::spawn(server);

    // open browser
    tokio::task::spawn_blocking(move || opener::open_browser(url.as_ref())).await??;

    // wait for response
    let received_data;
    loop {
        info!("Waiting for token from login process...");
        match rx.recv().await {
            Some(auth_data) => {
                received_data = auth_data;
                break;
            }
            None => {
                info!("Auth data channel was closed before receiving anything...");
                exit(1);
            }
        }
    }
    info!("Received data: {:#?}", received_data);

    // signal shutdown to server
    server_shutdown_token.cancel();

    // handle errors
    if let Some(expected_realm) = realm
        && *expected_realm.as_ref() != received_data.realm_id
    {
        error!(
            "Received realm id didn't match expected realm id. Received realm id: {}",
            received_data.realm_id
        );
        exit(1);
    }

    // wait for server to finish shutting down
    t1.await??;

    // return response
    Ok(received_data)
}

pub async fn create_oauth_client_without_redirect<S: AsRef<str>>(
    environment: &AuthEnvironment,
    client_id: S,
    client_secret: S,
) -> Result<Client<QBSandboxProvider>, anyhow::Error> {
    create_oauth_client_with_redirect(environment, None, None, client_id, client_secret).await
}

pub async fn create_oauth_client_with_redirect<S: AsRef<str>>(
    environment: &AuthEnvironment,
    listen_host: Option<&str>,
    listen_port: Option<u16>,
    client_id: S,
    client_secret: S,
) -> Result<Client<QBSandboxProvider>, anyhow::Error> {
    let client_id = client_id.as_ref();
    let client_secret = client_secret.as_ref();

    let http_client = reqwest::Client::new();

    let config = match environment {
        AuthEnvironment::Sandbox => qb_provider_config(SANDBOX_DISC_URL).await?,
        AuthEnvironment::Production => qb_provider_config(PRODUCTION_DISC_URL).await?,
    };

    let jwks = jwks(&http_client, config.jwks_uri.clone()).await?;
    debug!("jwks: {:#?}", jwks);

    let provider = config.clone().into();
    debug!("Provider: {:#?}", provider);

    let redirect = match (listen_host, listen_port) {
        (Some(h), Some(p)) => Some(format!("https://{}:{}", h, p)),
        (None, None) => None,
        _ => return Err(anyhow::anyhow!("need host and port")),
    };

    let oauth_client: Client<_, StandardClaims> = openid::Client::new(
        provider,
        client_id.into(),
        Some(client_secret.into()),
        redirect,
        reqwest::Client::new(),
        Some(jwks),
    );
    debug!("Oauth client: {:#?}", oauth_client);

    Ok(oauth_client)
}

const SECRET_SERVICE_SERVER: &'static str = "qbcli";

pub async fn store_access_key<'a>(
    keyring: &Keyring,
    profile: &str,
    environment: &str,
    realm_id: impl AsRef<str>,
    token: &Token<StandardClaims>,
) -> Result<()> {
    let realm_id = realm_id.as_ref();
    debug!("Realm id: {realm_id}");

    debug!("Bearer: {:#?}", token.bearer);

    let mut attributes = HashMap::from([("server", SECRET_SERVICE_SERVER), ("profile", profile)]);

    let items = keyring.search_items(&attributes).await?;
    debug!("items: {:#?}", items);

    attributes.extend([("environment", environment), ("realm", realm_id)]);

    match items.len() {
        n if n == 0 => {}
        n if n == 1 => {
            // check to make sure it's the same
            let item = &items[0];
            let found_attributes = item.attributes().await?;
            if found_attributes.len() != attributes.len() {
                return Err(anyhow::anyhow!(
                    "Profile already exists with different number of attributes. Can't create profile."
                ));
            }
            if !attributes
                .iter()
                .all(|(key, value)| found_attributes.get(*key).map_or(false, |v| *value == *v))
            {
                println!("found_attributes: {found_attributes:#?}");
                println!("attributes: {attributes:#?}");
                return Err(anyhow::anyhow!(
                    "Existing profile found with different attrbutes. Can't create profile ({}).",
                    profile
                ));
            };
        }
        n if n > 1 => {
            return Err(anyhow::anyhow!(
                "Multiple entries exist that match the profile. Cannot create item."
            ));
        }
        _ => unreachable!(),
    }

    for i in items {
        debug!("{}", i.label().await?);
    }

    let secret = toml::to_string(&token.bearer)?;

    debug!("attributes: {attributes:#?}");
    debug!("secret: {secret:#?}");

    keyring
        .create_item(
            format!("Token/{}", profile).as_ref(), // label
            &attributes,
            secret, //secret
            true,   // replace item with same attributes
        )
        .await?;

    Ok(())
}

pub async fn get_stored_profile_auth_data<'a>(
    keyring: &Keyring,
    profile_name: &str,
) -> Result<Option<Item>> {
    let attributes = &[("profile", profile_name), ("server", SECRET_SERVICE_SERVER)];
    let mut search = keyring.search_items(attributes).await?;
    if search.len() > 1 {
        return Err(anyhow::anyhow!(
            "Found multiple entries for profile ({}).",
            profile_name
        ));
    }

    Ok(match search.first() {
        Some(item) => {
            println!("{:#?}", item.label().await);

            // let mut join_set = tokio::task::JoinSet::new();
            // for t in search.into_iter() {
            //     join_set.spawn(async move { t.get_label() });
            // }
            let data = search.swap_remove(0);
            Some(data)
        }
        None => None,
    })
}
