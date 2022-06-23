use anyhow::Result;
use chrono::prelude::*;
use config::Configuration;
pub fn new_config(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    Configuration::new(config_file_path.as_str(), false)?;
    Ok(())
}

pub fn export_as_json(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    let config = Configuration::load(config_file_path.as_str(), false)?;
    let name_parts: Vec<&str> = config_file_path.split('.').collect();
    let mut name = String::new();
    name.push_str(name_parts[0]);
    name.push_str(".json");
    config.save(name.as_str(), true)?;
    Ok(())
}

pub fn fix(config_file_path: String) -> Result<()> {
    let mut config = Configuration::load(config_file_path.as_str(), false)?;
    config.fix();
    config.save(&config_file_path, false)?;
    Ok(())
}

pub fn seed_database(config_file_path: String) -> Result<()> {
    let config = Configuration::load(config_file_path.as_str(), false)?;
    let rpc_client = config.rpc_client();
    let db = tulip_realms_sdk::Database::new(config.db_opts)?;
    db.populate_database_with_mint_governance(
        config.realm_info.realm_key(),
        config.realm_info.council_mint_key(),
        config.realm_info.community_mint_key(),
        Utc::now(),
        &rpc_client,
    )
    .unwrap();
    Ok(())
}
