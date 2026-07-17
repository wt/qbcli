use anyhow::Result;
use tracing::info;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

pub(crate) fn setup_logging(
    conf_env_filter: Option<impl AsRef<str>>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    info!("Setting up logging...");
    let env_filter = match quiet {
        true => EnvFilter::new(""),
        false => match verbose {
            0 => match std::env::var(EnvFilter::DEFAULT_ENV) {
                Ok(_) => EnvFilter::from_default_env(),
                Err(_) => match conf_env_filter {
                    Some(s) => EnvFilter::new(s),
                    None => EnvFilter::default(),
                },
            },
            1 => EnvFilter::default().add_directive(LevelFilter::WARN.into()),
            2 => EnvFilter::default().add_directive(LevelFilter::INFO.into()),
            3 => EnvFilter::default()
                .add_directive(LevelFilter::INFO.into())
                .add_directive(format!("{}=debug", env!("CARGO_CRATE_NAME")).parse()?),
            4 => EnvFilter::default().add_directive(LevelFilter::DEBUG.into()),
            5 => EnvFilter::default()
                .add_directive(LevelFilter::DEBUG.into())
                .add_directive(format!("{}=trace", env!("CARGO_CRATE_NAME")).parse()?),
            i if i > 5 => {
                info!("Maximum verbosity (>4) set)");
                EnvFilter::default().add_directive(LevelFilter::TRACE.into())
            }
            _ => panic!("How did this happen?"),
        },
    };

    info!("logging EnvFilter: {}", env_filter);

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(env_filter)
        .init();
    Ok(())
}
