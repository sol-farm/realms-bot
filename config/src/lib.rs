use anyhow::Result;
use serde::{Deserialize, Serialize};
use simplelog::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{read_keypair_file, Keypair};
use std::fs;
use std::fs::File;
/// main configuration object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub discord: Discord,
    pub db_url: String,
    pub log_file: String,
    pub debug_log: bool,
    pub rpc_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discord {
    /// the main account functioning as the DAO
    pub dao_account: String,
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
    pub fn new(path: &str, as_json: bool) -> Result<()> {
        let config = Configuration::default();
        config.save(path, as_json)
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
                dao_account: "413KSeuFUBSWDzfjU9BBqBAWYKmoR8mncrhV84WcGNAk".to_string(),
                bot_token: "".to_string(),
                worker_loop_frequency: 600,
                status_channel: 0,
            },
            db_url: "postgres://postgres:necc@postgres/kek".to_string(),
            log_file: "template.log".to_string(),
            debug_log: false,
            rpc_url: "https://solana-api.projectserum.com".to_string(),
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
