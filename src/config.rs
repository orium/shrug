use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no config directory available")]
    NoConfigDir,
    #[error("failed to read config file: {0}")]
    ReadFile(std::io::Error),
    #[error("failed to create config directory: {0}")]
    CreateDir(std::io::Error),
    #[error("failed to write default config: {0}")]
    WriteFile(std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug)]
pub struct Alias<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

impl<'a> Alias<'a> {
    fn new(key: &'a str, value: &'a str) -> Alias<'a> {
        Alias { key, value }
    }
}

#[derive(serde_derive::Deserialize, Debug)]
pub struct Config {
    // WIP! make this a vector
    aliases: BTreeMap<String, String>,
}

impl Config {
    pub fn from_config_file() -> Result<Config, ConfigError> {
        let config_file_path = Config::config_file_path()?;
        let config_string =
            std::fs::read_to_string(config_file_path).map_err(ConfigError::ReadFile)?;
        Config::from_toml(&config_string)
    }

    fn from_toml(string: &str) -> Result<Config, ConfigError> {
        Ok(toml::from_str::<Config>(string)?)
    }

    pub fn create_file_if_nonexistent() -> Result<(), ConfigError> {
        let config_file_path = Config::config_file_path()?;

        if !config_file_path.exists() {
            let default_config = include_str!("default-config.toml");
            let parent = config_file_path.parent().ok_or(ConfigError::NoConfigDir)?;
            std::fs::create_dir_all(parent).map_err(ConfigError::CreateDir)?;
            std::fs::write(config_file_path, default_config).map_err(ConfigError::WriteFile)?;
        }

        Ok(())
    }

    fn config_file_path() -> Result<PathBuf, ConfigError> {
        let mut config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        config_dir.push("shrug");
        config_dir.push("shrug.toml");
        Ok(config_dir)
    }

    pub fn aliases(&self) -> impl Iterator<Item = Alias<'_>> {
        self.aliases.iter().map(|(k, v)| Alias::new(k, v))
    }
}
