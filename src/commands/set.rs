use anyhow::Result;
use directories::ProjectDirs;
use tracing::{debug, info};

use crate::config::read_config_data_from_config_file;

#[derive(Debug, clap::Args)]
pub(crate) struct SetArgs {
    #[command(subcommand)]
    subcommand: SetSubCommands,
}

#[derive(Debug, clap::Subcommand)]
enum SetSubCommands {
    #[command(about = "Alter the default profile.")]
    DefaultProfile(DefaultProfileArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct DefaultProfileArgs {
    #[arg(help = "The profile to set. If no profile is given, the current setting will be shown.")]
    profile: Option<String>,
}

pub(crate) async fn do_set(set_args: &SetArgs, project_dirs: &ProjectDirs) -> Result<()> {
    match &set_args.subcommand {
        SetSubCommands::DefaultProfile(default_profile_args) => {
            do_default_profile_subcommand(default_profile_args, project_dirs).await?
        }
    };

    Ok(())
}

async fn do_default_profile_subcommand(
    default_profile_args: &DefaultProfileArgs,
    project_dirs: &ProjectDirs,
) -> Result<()> {
    let auth_data = read_config_data_from_config_file(&project_dirs)?;

    match &default_profile_args.profile {
        Some(profile) => {
            debug!("Setting default profile to ({})", profile,);
            let mut auth_data = auth_data;
            auth_data.set_default_profile(profile);
            info!("The current default profile is now ({}).", profile);
            let out_data = toml::to_string(&auth_data)?;
            info!("blah: {}", out_data);
        }
        None => {
            match auth_data.default_profile() {
                Some(profile) => {
                    info!("The current default profile is ({}).", profile)
                }
                None => info!("No default profile is set."),
            };
            let out_data = toml::to_string(&auth_data)?;
            info!("blah: {}", out_data);
        }
    }
    Ok(())
}
