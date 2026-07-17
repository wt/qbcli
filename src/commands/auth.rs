use anyhow::Result;
use directories::ProjectDirs;
use oo7::Keyring;
use openid::Token;
use tracing::{debug, info};

use crate::config::get_oauth_creds;
use crate::util::auth::{
    QBAuthData, create_oauth_client_with_redirect, create_oauth_client_without_redirect,
    get_qb_auth_data, get_stored_profile_auth_token, store_access_key,
};
use crate::util::cli::ProfileArgs;
use crate::{config::read_config_data_from_config_file, util::AuthEnvironment};

#[derive(Debug, clap::Args)]
pub(crate) struct AuthArgs {
    #[command(subcommand)]
    subcommand: SubCommands,
}

#[derive(Debug, clap::Subcommand)]
enum SubCommands {
    #[command(about = "Login to acquire an access token.")]
    Login(LoginArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct LoginArgs {
    #[arg(long, help = "Disable token refresh. Require fetching new token.")]
    disable_token_refresh: bool,

    #[arg(
        long,
        default_value = "localhost",
        help = "Hostname to listen for auth redirect response."
    )]
    listen_host: String,

    #[arg(
        long,
        default_value_t = 9999,
        help = "Port to listen for auth reddirect response."
    )]
    listen_port: u16,

    #[arg(
        short('e'),
        value_enum,
        default_value_t = AuthEnvironment::Sandbox,
        help = "Quickbooks environment for auth"
    )]
    environment: AuthEnvironment,

    #[command(flatten)]
    profile_args: ProfileArgs,

    #[arg(short('r'), long)]
    realm: Option<String>,
}

pub(crate) async fn do_auth(auth_args: &AuthArgs, project_dirs: &ProjectDirs) -> Result<()> {
    match auth_args.subcommand {
        SubCommands::Login(ref login_args) => do_login(login_args, &project_dirs).await?,
    }
    Ok(())
}

pub(crate) async fn do_login(login_args: &LoginArgs, project_dirs: &ProjectDirs) -> Result<()> {
    let config = read_config_data_from_config_file(&project_dirs)?;
    let profile = login_args.profile_args.profile(&config);
    info!("profile name: {profile}");

    let keyring = Keyring::new().await?;

    let app_creds = get_oauth_creds(&login_args.environment)?;

    let oauth_client = create_oauth_client_with_redirect(
        &login_args.environment,
        Some(&login_args.listen_host),
        Some(login_args.listen_port),
        &app_creds.client_id,
        &app_creds.client_secret,
    )
    .await?;

    let mut qb_auth_data = match login_args.disable_token_refresh {
        true => request_new_token(login_args, &oauth_client).await?,
        false => match get_stored_profile_auth_token(&keyring, &profile).await? {
            Some(mut data) => {
                let oauth_client = create_oauth_client_without_redirect(
                    &data.environment,
                    &app_creds.client_id,
                    &app_creds.client_secret,
                )
                .await?;
                data.refresh_token(&oauth_client).await?;
                data
            }
            None => request_new_token(login_args, &oauth_client).await?,
        },
    };

    // This must happen every time in order for requesting userinfo to work.
    oauth_client.decode_token(
        &mut qb_auth_data
            .token
            .id_token
            .as_mut()
            .expect("Id token doesn't exist in token. Please login without token refresh."),
    )?;
    let qb_auth_data = qb_auth_data;
    debug!("decoded token id_token: {:#?}", qb_auth_data.token.id_token);
    info!("realm: {}", qb_auth_data.realm);

    let userinfo = oauth_client.request_userinfo(&qb_auth_data.token).await?;
    info!("userinfo: {userinfo:#?}");

    info!("environment: {:}", login_args.environment.arg_string());
    // info!("userinfo email: {:#?}", userinfo.email);
    info!("bearer ...");

    store_access_key(
        &keyring,
        &profile,
        &login_args.environment.arg_string(),
        &qb_auth_data.realm,
        &qb_auth_data.token,
    )
    .await?;

    Ok(())
}

async fn request_new_token(
    login_args: &LoginArgs,
    oauth_client: &openid::Client<crate::util::auth::QBSandboxProvider>,
) -> Result<QBAuthData, anyhow::Error> {
    let received_data = get_qb_auth_data(
        oauth_client,
        &login_args.listen_host,
        login_args.listen_port,
        login_args.realm.as_ref(),
    )
    .await?;
    info!("Getting a token...");
    let bearer = oauth_client
        .request_token(&received_data.code)
        .await
        .unwrap();
    info!("bearer: {:#?}", bearer);
    let realm_id = received_data.realm_id;
    let token: Token<_> = Token::from(bearer);
    debug!("decoded token bearer: {:#?}", token.bearer);
    Ok(QBAuthData {
        environment: login_args.environment.to_owned(),
        realm: realm_id,
        token,
    })
}
