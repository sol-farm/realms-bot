use anyhow::Result;
use serde::{Deserialize, Serialize};
use simplelog::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use std::str::FromStr;
use std::fs;
use std::fs::File;
/// main configuration object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub discord: Discord,
    pub db_opts: tulip_sled_util::config::DbOpts,
    /// information for a particular realms configuration, only supporting mint based governance
    pub realm_info: RealmsConfig,
    pub log_file: String,
    pub debug_log: bool,
    pub rpc_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RealmsConfig {
    pub realm_key: String,
    pub council_mint_key: String,
    pub community_mint_key: String,
    pub governance_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discord {
    /// the discord bot token
    pub bot_token: String,
    /// the channel to post messages too
    pub status_channel: u64,
    /// how often the workloop should run
    /// which is responsible for things such as automated
    /// check ins, etc..
    pub worker_loop_frequency: u64,
}

impl Configuration {
    pub fn new(path: &str, as_json: bool) -> Result<Self> {
        let config = Configuration::default();
        config.save(path, as_json)?;
        Ok(config)
    }
    pub fn save(&self, path: &str, as_json: bool) -> Result<()> {
        let data = if as_json {
            serde_json::to_string_pretty(&self)?
        } else {
            serde_yaml::to_string(&self)?
        };
        fs::write(path, data).expect("failed to write to file");
        Ok(())
    }
    pub fn load(path: &str, from_json: bool) -> Result<Configuration> {
        let data = fs::read(path).expect("failed to read file");
        let config: Configuration = if from_json {
            serde_json::from_slice(data.as_slice())?
        } else {
            serde_yaml::from_slice(data.as_slice())?
        };
        Ok(config)
    }
    pub fn rpc_client(&self) -> RpcClient {
        RpcClient::new(self.rpc_url.to_string())
    }
    pub fn fix(&mut self) {
        self.realm_info.fix();
    }
    /// if file_log is true, log to both file and stdout
    /// otherwise just log to stdout
    pub fn init_log(&self, file_log: bool) -> Result<()> {
        if !file_log {
            if self.debug_log {
                TermLogger::init(
                    LevelFilter::Debug,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Debug)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                )?;
                return Ok(());
            } else {
                TermLogger::init(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                )?;
                return Ok(());
            }
        }
        if self.debug_log {
            CombinedLogger::init(vec![
                TermLogger::new(
                    LevelFilter::Debug,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Debug)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Debug,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Debug)
                        .build(),
                    File::create(self.log_file.as_str()).unwrap(),
                ),
            ])?;
        } else {
            CombinedLogger::init(vec![
                TermLogger::new(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .build(),
                    File::create(self.log_file.as_str()).unwrap(),
                ),
            ])?;
        }

        Ok(())
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            discord: Discord {
                bot_token: "".to_string(),
                worker_loop_frequency: 600,
                status_channel: 0,
            },
            log_file: "template.log".to_string(),
            debug_log: false,
            rpc_url: "https://solana-api.projectserum.com".to_string(),
            db_opts: Default::default(),
            realm_info: Default::default(),
        }
    }
}


impl RealmsConfig {
    pub fn realm_key(&self) -> Pubkey {
        Pubkey::from_str(&self.realm_key).unwrap()
    }
    pub fn council_mint_key(&self) -> Pubkey {
        Pubkey::from_str(&self.council_mint_key).unwrap()
    }
    pub fn community_mint_key(&self) -> Pubkey {
        Pubkey::from_str(&self.community_mint_key).unwrap()
    }
    pub fn governance_key(&self) -> Pubkey {
        Pubkey::from_str(&self.governance_key).unwrap()
    }
    // attempts to "fix" the configuration by populating the governance address
    pub fn fix(&mut self) {
        if !self.realm_key.is_empty() && !self.council_mint_key.is_empty() {
            self.governance_key = tulip_realms_sdk::spl_governance::state::governance::get_mint_governance_address(
                &tulip_realms_sdk::GOVERNANCE_PROGRAM,
                &self.realm_key(),
                &self.council_mint_key()
            ).to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
