/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::BTreeMap;
use std::path::PathBuf;

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
    pub fn from_config_file() -> Config {
        let config_file_path = Config::config_file_path();

        let config_string =
            std::fs::read_to_string(config_file_path).expect("Failed to read configuration file");

        Config::from_toml(&config_string)
    }

    fn from_toml(string: &str) -> Config {
        toml::from_str::<Config>(string).expect("Failed to parse default configuration file")
    }

    pub fn create_file_if_nonexistent() {
        let config_file_path = Config::config_file_path();

        if !config_file_path.exists() {
            let default_config: &str = include_str!("default-config.toml");

            std::fs::create_dir_all(config_file_path.parent().unwrap())
                .expect("Failed to create the default configuration file");

            std::fs::write(config_file_path, default_config)
                .expect("Unable to write default configuration file");
        }
    }

    fn config_file_path() -> PathBuf {
        let mut config_dir: PathBuf = dirs::config_dir().expect("Unable to get config directory");

        config_dir.push("shrug");
        config_dir.push("shrug.toml");

        config_dir
    }

    pub fn aliases<'a>(&'a self) -> impl Iterator<Item = Alias<'a>> {
        self.aliases.iter().map(|(k, v)| Alias::new(k, v))
    }
}
