pub mod auth;
pub mod cli;

use std::cell::LazyCell;

use clap::ValueEnum as _;
use url::Url;

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum AuthEnvironment {
    Sandbox,
    Production,
}

const SANDBOX_URL_BASE: LazyCell<Url> = LazyCell::new(|| {
    Url::parse("https://sandbox-quickbooks.api.intuit.com/").expect("Bad base sandbox Url")
});
const PRODUCTION_URL_BASE: LazyCell<Url> = LazyCell::new(|| {
    Url::parse("https://quickbooks.api.intuit.com/").expect("Bad base sandbox Url")
});

impl AuthEnvironment {
    pub fn arg_string(&self) -> String {
        self.to_possible_value().unwrap().get_name().to_owned()
    }

    pub fn rest_base_url(&self) -> Url {
        match &self {
            AuthEnvironment::Sandbox => (*SANDBOX_URL_BASE).clone(),
            AuthEnvironment::Production => (*PRODUCTION_URL_BASE).clone(),
        }
    }
}
