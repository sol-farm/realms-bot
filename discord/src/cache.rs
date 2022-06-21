//! disk backed cache using sled

use std::sync::Arc;
use anyhow::{Result, anyhow};
use borsh::BorshSerialize;
use spl_governance::{state::{governance::GovernanceV2, proposal::ProposalV2, realm::RealmV2}, solana_program::pubkey::Pubkey};
/// Database is the main embedded database object using sled db
#[derive(Clone)]
pub struct Database {
    pub db: sled::Db,
}


impl Database {
    pub fn new(db_path: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self{db: sled::open(db_path)?}))
    }
    pub fn insert_governance(&self, key: Pubkey, governance: GovernanceV2) -> Result<()> {
        self.db.insert(
            key.to_bytes(),
            governance.try_to_vec()?,
        )?;
        Ok(())
    }
    pub fn insert_proposal(&self, key: Pubkey, proposal: ProposalV2) -> Result<()> {
        self.db.insert(
            key.to_bytes(),
            proposal.try_to_vec()?,
        )?;
        Ok(())
    }
    pub fn insert_realm(&self, key: Pubkey, realm: RealmV2) -> Result<()> {
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
    use solana_program::pubkey::Pubkey;
    use super::*;
    use spl_governance as realms_sdk;
    use solana_client::rpc_client::RpcClient;
    use solana_program::account_info::IntoAccountInfo;

    fn get_dao_account() -> Pubkey {
        Pubkey::from_str("413KSeuFUBSWDzfjU9BBqBAWYKmoR8mncrhV84WcGNAk").unwrap()
    }
    fn get_gov_prog() -> Pubkey {
        Pubkey::from_str("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw").unwrap()
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database() {
        let rpc = RpcClient::new("https://ssc-dao.genesysgo.net".to_string());
        let dao_key = get_dao_account();
        let gov_prog = get_gov_prog();
        let dao_account = rpc.get_account(&dao_key).unwrap();
        println!("data len {}", dao_account.data.len());
        let mut dao_account_tup = (dao_key, dao_account);
        let dao_account_info = dao_account_tup.into_account_info();
        let governance_account = realms_sdk::state::realm::get_realm_data(&gov_prog, &dao_account_info).unwrap();
        
        let db = Database::new("test_db").unwrap();

//        db.insert_realm(dao_key, governance_account).unwrap();

        std::fs::remove_dir_all("test_db").unwrap();
    }
}
