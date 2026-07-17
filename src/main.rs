mod commands;
mod config;
mod logging;
pub(crate) mod util;

use anyhow::Result;
use clap::Parser;
use directories::ProjectDirs;

use crate::commands::accounting::do_accounting;

#[derive(Debug, Parser)]
/// Quickbook Online API CLI tool.
///
/// A tool to send requests to the Quickbook Online APIs.
///
/// This tool is maintained at https://github.com/wt/qbcli. Find help there.
struct Args {
    #[command(flatten)]
    log_args: LogArgs,

    #[command(subcommand)]
    subcommand: SubCommands,
}

#[derive(Debug, clap::Args)]
#[group(multiple = false)]
struct LogArgs {
    #[arg(short, long)]
    quiet: bool,

    #[arg(short, long, action = clap::ArgAction::Count )]
    verbose: u8,
}

#[derive(Debug, clap::Subcommand)]
enum SubCommands {
    #[command(about = "Use the Quickbooks Online Accounting API.")]
    Accounting(commands::accounting::AccountingArgs),
    #[command(about = "Login and get access token for Quickbooks Online API.")]
    Auth(commands::auth::AuthArgs),
    #[command(about = "Alter settings.")]
    Set(commands::set::SetArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let project_dirs = ProjectDirs::from("com.ymatyt", "YMATYT Holdings, LLC", "Quickbooks CLI")
        .expect("Could not find valid home directory path.");
    dotenvy::dotenv()?;

    let args = Args::parse();

    // create bootstrap tracing subscriber
    let env_filter = tracing_subscriber::filter::EnvFilter::new("debug");
    let bootstrap_subscriber = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(env_filter)
        .finish();

    // set bootstrap tracing provider to default during reading of config and setup of global
    // logging config
    // The following line fails and has be filed as a bug with the tracing-subcriber
    //     crate:
    //     use tracing_subscriber::util::SubscriberInitExt;
    //     let bootstrap_log_sub_guard = bootstrap_subscriber.set_default();
    // The bug is at https://github.com/tokio-rs/tracing/issues/2903
    let bootstrap_log_sub_guard = tracing::subscriber::set_default(bootstrap_subscriber);

    logging::setup_logging(Some("info"), args.log_args.quiet, args.log_args.verbose)?;
    drop(bootstrap_log_sub_guard);
    // The bootstrap logging is done here.

    match args.subcommand {
        SubCommands::Accounting(accounting_args) => {
            do_accounting(&accounting_args, &project_dirs).await?
        }
        SubCommands::Auth(auth_args) => {
            // info!(
            //     "blah: {:#?}",
            //     config.extract_inner::<Config>("default_realm")?
            // );
            // std::process::exit(1);
            commands::auth::do_auth(&auth_args, &project_dirs).await?
        }
        SubCommands::Set(set_args) => commands::set::do_set(&set_args, &project_dirs).await?,
    }

    Ok(())
}
