use std::borrow::Cow;
use std::io::ErrorKind;
use std::io::Read as _;
use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::util::AuthEnvironment;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigData<'a> {
    default_profile: Option<Cow<'a, str>>,
}

impl<'a> ConfigData<'a> {
    pub fn default_profile(&self) -> Option<&str> {
        self.default_profile.as_ref().map(|x| x.as_ref())
    }

    pub fn set_default_profile(&mut self, profile: impl Into<Cow<'a, str>>) {
        self.default_profile = Some(profile.into());
    }
}

// #[derive(Debug, Deserialize, Serialize)]
// pub struct AuthEntry<'a> {
//     environment: Cow<'a, str>,
//     realm: Cow<'a, str>,
//     email: Cow<'a, str>,
//     bearer_token: Bearer,
// }

const CONFIG_DATA_PATH: &str = "config.toml";

pub fn config_file_path(project_dirs: &ProjectDirs) -> PathBuf {
    let mut config_path = project_dirs.config_dir().to_path_buf();

    config_path.push(CONFIG_DATA_PATH);
    config_path
}

pub fn read_config_data_from_config_file(project_dirs: &ProjectDirs) -> Result<ConfigData<'_>> {
    let auth_data_path = config_file_path(project_dirs);

    let auth_data = match std::fs::File::open(&auth_data_path) {
        Ok(mut f) => {
            let mut buf = String::new();
            match f.read_to_string(&mut buf) {
                Ok(_) => toml::from_str::<ConfigData>(buf.as_str())?,
                Err(_) => ConfigData::default(),
            }
        }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => ConfigData::default(),
            _ => unimplemented!(),
        },
    };
    debug!("auth_data: {auth_data:#?}");
    Ok(auth_data)
}

pub struct OAuthClientCredentials {
    pub client_id: String,
    pub client_secret: String,
}

pub fn get_oauth_sandbox_creds() -> Result<OAuthClientCredentials> {
    let client_id = std::env::var("SANDBOX_CLIENT_ID")?;
    let client_secret = std::env::var("SANDBOX_CLIENT_SECRET")?;
    Ok(OAuthClientCredentials {
        client_id,
        client_secret,
    })
}

pub fn get_oauth_production_creds() -> Result<OAuthClientCredentials> {
    let client_id = std::env::var("CLIENT_ID")?;
    let client_secret = std::env::var("CLIENT_SECRET")?;
    Ok(OAuthClientCredentials {
        client_id,
        client_secret,
    })
}

pub fn get_oauth_creds(environment: &AuthEnvironment) -> Result<OAuthClientCredentials> {
    match environment {
        AuthEnvironment::Sandbox => get_oauth_sandbox_creds(),
        AuthEnvironment::Production => get_oauth_production_creds(),
    }
}
