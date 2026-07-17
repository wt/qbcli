use anyhow::Result;
use directories::ProjectDirs;
use oo7::Keyring;
use serde_json::Value;
use tracing::info;
use url::Url;

use crate::{
    config::{ConfigData, get_oauth_creds, read_config_data_from_config_file},
    util::{
        auth::{create_oauth_client_without_redirect, get_stored_profile_auth_token},
        cli::ProfileArgs,
    },
};

#[derive(Debug, clap::Args)]
pub(crate) struct AccountingArgs {
    #[command(flatten)]
    profile_args: ProfileArgs,

    #[command(subcommand)]
    subcommand: SubCommands,
}

#[derive(Debug, clap::Subcommand)]
enum SubCommands {
    #[command(about = "Login and get access token for Quickbooks Online API.")]
    CompanyInfo,
}

pub async fn do_accounting(
    accounting_args: &AccountingArgs,
    project_dirs: &ProjectDirs,
) -> Result<()> {
    let config = read_config_data_from_config_file(&project_dirs)?;

    match accounting_args.subcommand {
        SubCommands::CompanyInfo => do_company_info(&accounting_args, &config).await?,
    }
    Ok(())
}

pub async fn do_company_info(
    accounting_args: &AccountingArgs,
    config: &ConfigData<'_>,
) -> Result<()> {
    let profile = accounting_args.profile_args.profile(&config);
    let keyring = Keyring::new().await?;

    if let Some(mut qb_auth_data) = get_stored_profile_auth_token(&keyring, profile).await? {
        let app_creds = get_oauth_creds(&qb_auth_data.environment)?;

        let oauth_client = create_oauth_client_without_redirect(
            &qb_auth_data.environment,
            &app_creds.client_id,
            &app_creds.client_secret,
        )
        .await?;

        qb_auth_data.refresh_token(&oauth_client).await?;
        let access_token = qb_auth_data.token.bearer.access_token;

        let base_url = qb_auth_data.environment.rest_base_url();
        let url = base_url.join(
            format!(
                "{}{}/companyinfo/{}",
                "/v3/company/", &qb_auth_data.realm, &qb_auth_data.realm
            )
            .as_ref(),
        )?;
        info!("url to fetch: {url}");

        let client = reqwest::Client::new();
        let resp = client
            .get(Url::parse(url.as_str()).unwrap())
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            // .header("accept", "application/json")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let bytes = resp.bytes().await?;
        let body_json: Value = serde_json::from_slice(&bytes)?;
        // info!("bytes: {:#?}", resp.bytes().await.unwrap());
        info!("body: {}", serde_json::to_string_pretty(&body_json)?);

        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Couldn't get the auth token from the secret storage."
        ))
    }
}
