use crate::config::ConfigData;

#[derive(Debug, clap::Args)]
pub struct ProfileArgs {
    #[arg(short('p'), long, default_value = "default", help = "Profile.")]
    pub profile: Option<String>,
}

impl ProfileArgs {
    pub fn profile<'a>(&'a self, config_data: &'a ConfigData<'a>) -> &'a str {
        self.profile.as_ref().map_or_else(
            || config_data.default_profile().unwrap_or_else(|| "default"),
            |x| x,
        )
    }
}
