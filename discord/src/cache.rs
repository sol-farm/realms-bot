//! disk backed cache using sled

use std::sync::Arc;
use anyhow::{Result, anyhow};
use borsh::BorshSerialize;
use realms_sdk::{state::{governance::Governance, proposal::Proposal, realm::Realm}, solana_program::pubkey::Pubkey};

/// Database is the main embedded database object using sled db
#[derive(Clone)]
pub struct Database {
    pub db: sled::Db,
}


impl Database {
    pub fn new(db_path: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self{db: sled::open(db_path)?}))
    }
    pub fn insert_governance(&self, key: Pubkey, governance: Governance) -> Result<()> {
        self.db.insert(
            key.to_bytes(),
            governance.try_to_vec()?,
        )?;
        Ok(())
    }
    pub fn insert_proposal(&self, key: Pubkey, proposal: Proposal) -> Result<()> {
        self.db.insert(
            key.to_bytes(),
            proposal.try_to_vec()?,
        )?;
        Ok(())
    }
    pub fn insert_realm(&self, key: Pubkey, realm: Realm) -> Result<()> {
        self.db.insert(
            key.to_bytes(),
            realm.try_to_vec()?,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use realms_sdk::solana_program::account_info::IntoAccountInfo;
    use solana_client::rpc_client::RpcClient;
    fn get_dao_account() -> Pubkey {
        Pubkey::from_str("413KSeuFUBSWDzfjU9BBqBAWYKmoR8mncrhV84WcGNAk").unwrap()
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database() {
        let rpc = RpcClient::new("https://ssc-dao.genesysgo.net".to_string());
        let dao_key = get_dao_account();
        let dao_account = rpc.get_account(&dao_key).unwrap();
        println!("data len {}", dao_account.data.len());
        println!("{}", std::mem::size_of::<realms_sdk::state::realm::Realm>());
        println!("{}", std::mem::size_of::<realms_sdk::state::governance::Governance>());
        println!("{}", std::mem::size_of::<realms_sdk::state::token_owner_record::TokenOwnerRecord>());
        let mut dao_account_tup = (dao_key, dao_account);
        let dao_account_info = dao_account_tup.into_account_info();
        let governance_account = realms_sdk::state::governance::get_governance_data(&dao_account_info).unwrap();
        
        let db = Database::new("test_db").unwrap();

//        db.insert_realm(dao_key, governance_account).unwrap();

        std::fs::remove_dir_all("test_db").unwrap();
    }
}
