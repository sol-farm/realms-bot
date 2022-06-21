//! disk backed cache using sled

use std::sync::Arc;
use anyhow::{Result, anyhow};
use borsh::{BorshSerialize, BorshDeserialize, BorshSchema};
use sled::IVec;
use solana_program::account_info::AccountInfo;
use spl_governance::{state::{governance::GovernanceV2, proposal::ProposalV2, realm::RealmV2}, solana_program::pubkey::Pubkey};
use static_pubkey::static_pubkey;
use tulip_sled_util::types::{DbKey, DbTrees};

pub const GOVERNANCE_TREE: &str = "governance_info";
pub const PROPOSAL_TREE: &str = "proposal_info";
pub const REALM_TREE: &str = "realm_info";
pub const GOVERNANCE_PROGRAM: Pubkey = static_pubkey!("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw");

/// Database is the main embedded database object using sled db
#[derive(Clone)]
pub struct Database {
    pub db: Arc<tulip_sled_util::Database>,
}


impl Database {
    pub fn new(opts: tulip_sled_util::config::DbOpts) -> Result<Arc<Self>> {
        Ok(Arc::new(Self{db: tulip_sled_util::Database::new(&opts)?}))
    }
    pub fn insert_governance(&self, governance: GovernanceV2Wrapper) -> Result<()> {
        self.db.open_tree(DbTrees::Custom(GOVERNANCE_TREE))?.insert(&governance)?;
        Ok(())
    }
    pub fn insert_proposal(&self, proposal: ProposalV2Wrapper) -> Result<()> {
        self.db.open_tree(DbTrees::Custom(PROPOSAL_TREE))?.insert(&proposal)?;
        Ok(())
    }
    pub fn insert_realm(&self, realm: RealmV2Wrapper) -> Result<()> {
        self.db.open_tree(DbTrees::Custom(REALM_TREE))?.insert(&realm)?;
        Ok(())
    }
    pub fn list_governances(&self) -> Result<Vec<GovernanceV2Wrapper>> {
        let tree = self.db.open_tree(DbTrees::Custom(GOVERNANCE_TREE))?;
        let keys: Vec<IVec> = tree.iter().filter_map(|entry| {
            if let Ok((key, _)) = entry {
                Some(key)
            } else {
                None
            }
        }).collect();
        let govs = keys.iter().filter_map(|key| {
            let governance: GovernanceV2Wrapper = if let Ok(gov) = tree.deserialize(key) {
                gov
            } else {
                return None;
            };
            Some(governance)
        }).collect();
        Ok(govs)
    }
    pub fn list_proposals(&self) -> Result<Vec<ProposalV2Wrapper>> {
        let tree = self.db.open_tree(DbTrees::Custom(PROPOSAL_TREE))?;
        let keys: Vec<IVec> = tree.iter().filter_map(|entry| {
            if let Ok((key, _)) = entry {
                Some(key)
            } else {
                None
            }
        }).collect();
        let govs = keys.iter().filter_map(|key| {
            let proposal: ProposalV2Wrapper = if let Ok(gov) = tree.deserialize(key) {
                gov
            } else {
                return None;
            };
            Some(proposal)
        }).collect();
        Ok(govs)
    }
}


#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct GovernanceV2Wrapper {
    pub governance: GovernanceV2,
    pub key: Pubkey,
}
impl DbKey for GovernanceV2Wrapper {
    fn key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.key.to_bytes().to_vec())
    }
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct ProposalV2Wrapper {
    pub proposal: ProposalV2,
    pub key: Pubkey,
}

impl DbKey for ProposalV2Wrapper {
    fn key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.key.to_bytes().to_vec())
    }
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct RealmV2Wrapper {
    pub realm: RealmV2,
    pub key: Pubkey,
}

impl DbKey for RealmV2Wrapper {
    fn key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.key.to_bytes().to_vec())
    }
}


/// returns a RealmV2Wrapper if the account can be deserialized into a RealmV2 account
pub fn get_realm_wrapper(
    realm_account: &AccountInfo,
) -> Result<RealmV2Wrapper> {
    let realm_data = spl_governance::state::realm::get_realm_data(
        &GOVERNANCE_PROGRAM,
        realm_account
    )?;
    Ok(RealmV2Wrapper {
        realm: realm_data,
        key: *realm_account.key
    })
}


/// returns a ProposalV2Wrapper if the account can be deserialized into a ProposalV2 account
pub fn get_proposal_wrapper(
    proposal_account: &AccountInfo,
) -> Result<ProposalV2Wrapper> {
    let prop_data = spl_governance::state::proposal::get_proposal_data(
        &GOVERNANCE_PROGRAM,
        proposal_account
    )?;
    Ok(ProposalV2Wrapper {
        proposal: prop_data,
        key: *proposal_account.key
    })
}

/// returns a GovernanceV2Wrapper if the account can be deserialized into a ProposalV2 account
pub fn get_governance_wrapper(
    governance_account: &AccountInfo,
) -> Result<GovernanceV2Wrapper> {
    let gov_data = spl_governance::state::governance::get_governance_data(
        &GOVERNANCE_PROGRAM,
        governance_account
    )?;
    Ok(GovernanceV2Wrapper {
        governance: gov_data,
        key: *governance_account.key
    })
}


#[cfg(test)]
mod test {
    use std::str::FromStr;
    use solana_program::pubkey::Pubkey;
    use spl_governance::state::proposal::get_proposal_address;
    use super::*;
    use solana_client::rpc_client::RpcClient;
    use solana_program::account_info::IntoAccountInfo;

    fn get_tulip_realm_account() -> Pubkey {
        Pubkey::from_str("413KSeuFUBSWDzfjU9BBqBAWYKmoR8mncrhV84WcGNAk").unwrap()
    }
    fn get_tulip_council_mint() -> Pubkey {
        Pubkey::from_str("EzSjCzCPwpchdQVaGJZYpgDNagzasKFVGJ66Dmut26FL").unwrap()
    }
    fn get_tulip_community_mint() -> Pubkey {
        Pubkey::from_str("STuLiPmUCUtG1hQcwdc9de9sjYhVsYoucCiWqbApbpM").unwrap()
    }
    fn get_tulip_governance_account() -> Pubkey {
        spl_governance::state::governance::get_mint_governance_address(
            &GOVERNANCE_PROGRAM,
            &get_tulip_realm_account(),
            &get_tulip_council_mint()
        )
    }
    fn real_proposal_key() -> Pubkey {
        Pubkey::from_str("BMVK7TYccgHR1Akjb1Hr9qDAfxWCGZEMt8V1Fo2U4GVn").unwrap()
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn log_proposal_account() {
        let rpc = RpcClient::new("https://ssc-dao.genesysgo.net".to_string());
        let proposal_key = real_proposal_key();
        let proposal_account = rpc.get_account(&proposal_key).unwrap();
        let mut proposal_account_tup = (proposal_key, proposal_account);
        let proposal_account_info = proposal_account_tup.into_account_info();
        let proposal = get_proposal_wrapper(&proposal_account_info).unwrap();
        println!("{:#?}", proposal.proposal);

    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database() {
        let rpc = RpcClient::new("https://ssc-dao.genesysgo.net".to_string());
        
        let realm_key = get_tulip_realm_account();
        let realm_account = rpc.get_account(&realm_key).unwrap();
        let mut realm_account_tup = (realm_key, realm_account);
        let realm_account_info = realm_account_tup.into_account_info();
        let realm = get_realm_wrapper(&realm_account_info).unwrap();

        let main_gov_key = get_tulip_governance_account();
        let main_gov_account = rpc.get_account(&main_gov_key).unwrap();
        let mut main_gov_account_tup = (main_gov_key, main_gov_account);
        let main_gov_info = main_gov_account_tup.into_account_info();
        let main_gov = get_governance_wrapper(&main_gov_info).unwrap();

        let proposal1_key = get_proposal_address(
            &GOVERNANCE_PROGRAM,
            &main_gov_key,
            &get_tulip_community_mint(),
            &(0_u32.to_le_bytes())
        );
        let proposal1_account = rpc.get_account(&proposal1_key).unwrap();
        let mut proposal1_account_tup = (proposal1_key, proposal1_account);
        let proposal1_account_info = proposal1_account_tup.into_account_info();
        let proposal1 = get_proposal_wrapper(&proposal1_account_info).unwrap();

        let proposal2_key = get_proposal_address(
            &GOVERNANCE_PROGRAM,
            &main_gov_key,
            &get_tulip_community_mint(),
            &(1_u32.to_le_bytes())
        );
        let proposal2_account = rpc.get_account(&proposal2_key).unwrap();
        let mut proposal2_account_tup = (proposal2_key, proposal2_account);
        let proposal2_account_info = proposal2_account_tup.into_account_info();
        let proposal2 = get_proposal_wrapper(&proposal2_account_info).unwrap();


        let mut opts = tulip_sled_util::config::DbOpts::default();
        opts.path = "realms_sdk.db".to_string();

        let db = Database::new(opts).unwrap();

        db.insert_realm(realm).unwrap();
        db.insert_governance(main_gov).unwrap();
        db.insert_proposal(proposal1).unwrap();
        db.insert_proposal(proposal2).unwrap();

        let proposals = db.list_proposals().unwrap();
        assert_eq!(proposals.len(), 2);



        std::fs::remove_dir_all("realms_sdk.db").unwrap();
    }
}
