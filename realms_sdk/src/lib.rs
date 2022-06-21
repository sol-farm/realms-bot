//! disk backed cache for realms related accounts using sled

pub mod types;
pub mod utils;
use crate::utils::governance_notif_cache_key;
use anyhow::Result;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use chrono::prelude::*;
use sled::IVec;
use solana_client::rpc_client::RpcClient;
use solana_program::account_info::AccountInfo;
use solana_program::account_info::IntoAccountInfo;
use spl_governance::{
    solana_program::pubkey::Pubkey,
    state::{governance::GovernanceV2, proposal::ProposalV2, realm::RealmV2},
};
use static_pubkey::static_pubkey;
use std::sync::Arc;
use tulip_sled_util::types::{DbKey, DbTrees};
use types::NotifCacheEntry;
use types::{
    get_governance_wrapper, get_proposal_wrapper, get_realm_wrapper, GovernanceV2Wrapper,
    ProposalV2Wrapper, RealmV2Wrapper,
};

pub const GOVERNANCE_TREE: &str = "governance_info";
pub const PROPOSAL_TREE: &str = "proposal_info";
pub const REALM_TREE: &str = "realm_info";
pub const GOVERNANCE_PROGRAM: Pubkey =
    static_pubkey!("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw");

pub use spl_governance;

/// Database is the main embedded database object using sled db
#[derive(Clone)]
pub struct Database {
    pub db: Arc<tulip_sled_util::Database>,
}

impl Database {
    pub fn new(opts: tulip_sled_util::config::DbOpts) -> Result<Self> {
        Ok(Self {
            db: tulip_sled_util::Database::new(&opts)?,
        })
    }
    pub fn insert_governance(&self, governance: &GovernanceV2Wrapper) -> Result<()> {
        self.db
            .open_tree(DbTrees::Custom(GOVERNANCE_TREE))?
            .insert(governance)?;
        Ok(())
    }
    pub fn insert_proposal(&self, proposal: &ProposalV2Wrapper) -> Result<()> {
        self.db
            .open_tree(DbTrees::Custom(PROPOSAL_TREE))?
            .insert(proposal)?;
        Ok(())
    }
    pub fn insert_realm(&self, realm: &RealmV2Wrapper) -> Result<()> {
        self.db
            .open_tree(DbTrees::Custom(REALM_TREE))?
            .insert(realm)?;
        Ok(())
    }
    pub fn insert_notif_cache_entry(&self, cache_entry: &NotifCacheEntry) -> Result<()> {
        self.db.open_tree(DbTrees::Default)?.insert(cache_entry)?;
        Ok(())
    }
    pub fn get_governance_notif_cache(&self, governance_key: Pubkey) -> Result<NotifCacheEntry> {
        let notif_cache = self
            .db
            .open_tree(DbTrees::Default)?
            .deserialize(governance_notif_cache_key(governance_key))?;
        Ok(notif_cache)
    }
    pub fn list_governances(&self) -> Result<Vec<GovernanceV2Wrapper>> {
        let tree = self.db.open_tree(DbTrees::Custom(GOVERNANCE_TREE))?;
        let keys: Vec<IVec> = tree
            .iter()
            .filter_map(|entry| {
                if let Ok((key, _)) = entry {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        let govs = keys
            .iter()
            .filter_map(|key| {
                let governance: GovernanceV2Wrapper = if let Ok(gov) = tree.deserialize(key) {
                    gov
                } else {
                    return None;
                };
                Some(governance)
            })
            .collect();
        Ok(govs)
    }
    pub fn list_proposals(&self) -> Result<Vec<ProposalV2Wrapper>> {
        let tree = self.db.open_tree(DbTrees::Custom(PROPOSAL_TREE))?;
        let keys: Vec<IVec> = tree
            .iter()
            .filter_map(|entry| {
                if let Ok((key, _)) = entry {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        let props = keys
            .iter()
            .filter_map(|key| {
                let proposal: ProposalV2Wrapper = if let Ok(prop) = tree.deserialize(key) {
                    prop
                } else {
                    return None;
                };
                Some(proposal)
            })
            .collect();
        Ok(props)
    }
    pub fn list_realms(&self) -> Result<Vec<RealmV2Wrapper>> {
        let tree = self.db.open_tree(DbTrees::Custom(REALM_TREE))?;
        let keys: Vec<IVec> = tree
            .iter()
            .filter_map(|entry| {
                if let Ok((key, _)) = entry {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        let realms = keys
            .iter()
            .filter_map(|key| {
                let realm: RealmV2Wrapper = if let Ok(realm) = tree.deserialize(key) {
                    realm
                } else {
                    return None;
                };
                Some(realm)
            })
            .collect();
        Ok(realms)
    }
    /// given a realm key, populate the database with all related mint governance accounts, and proposals
    ///
    /// this will not be the most performant as every insert flushes and syncs to disk, so if maximal performance
    /// is desired you'll want to leverage batch transactions.
    pub fn populate_database_with_mint_governance(
        &self,
        // the realm account key
        realm_key: Pubkey,
        council_mint_key: Pubkey,
        community_mint_key: Pubkey,
        now: DateTime<Utc>,
        rpc: &RpcClient,
    ) -> Result<()> {
        let realm_account = rpc.get_account(&realm_key).unwrap();
        let mut realm_account_tup = (realm_key, realm_account);
        let realm_account_info = realm_account_tup.into_account_info();
        let realm = get_realm_wrapper(&realm_account_info).unwrap();
        self.insert_realm(&realm)?;

        let mint_gov_key = spl_governance::state::governance::get_mint_governance_address(
            &GOVERNANCE_PROGRAM,
            &realm_key,
            &council_mint_key,
        );
        let main_gov_account = rpc.get_account(&mint_gov_key).unwrap();
        let mut main_gov_account_tup = (mint_gov_key, main_gov_account);
        let main_gov_info = main_gov_account_tup.into_account_info();
        let mint_gov = get_governance_wrapper(&main_gov_info).unwrap();
        self.insert_governance(&mint_gov)?;

        let mut notif_cache = NotifCacheEntry {
            governance_key: mint_gov_key,
            last_proposals_count: mint_gov.governance.proposals_count,
            voting_proposals_last_notification_time: Vec::with_capacity(5),
        };

        // now parse over all existing proposals, inserting them into the database
        for idx in 0..mint_gov.governance.proposals_count {
            let proposal_key = spl_governance::state::proposal::get_proposal_address(
                &GOVERNANCE_PROGRAM,
                &mint_gov_key,
                &community_mint_key,
                &idx.to_le_bytes()[..],
            );
            let proposal_account = rpc.get_account(&proposal_key).unwrap();
            let mut proposal_account_tup = (proposal_key, proposal_account);
            let proposal_account_info = proposal_account_tup.into_account_info();
            let mut proposal = get_proposal_wrapper(&proposal_account_info).unwrap();
            // attempt to finalize vote if possible, as this may not always be done on-chain, even
            // if a vote has ended. really the only time this will likely be done on-chain is for a vote that is
            // completed
            proposal.finalize_vote(&mint_gov.governance.config, now);
            if proposal.proposal.voting_at.is_some()
                && !proposal.has_vote_time_ended(&mint_gov.governance.config, now)
            {
                notif_cache
                    .voting_proposals_last_notification_time
                    .push((proposal.key, 0));
            }

            self.insert_proposal(&proposal)?;
        }

        // insert the notif cache entry
        self.insert_notif_cache_entry(&notif_cache)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use solana_client::rpc_client::RpcClient;
    use solana_program::account_info::IntoAccountInfo;
    use solana_program::pubkey::Pubkey;
    use spl_governance::state::proposal::get_proposal_address;
    use std::str::FromStr;

    pub fn get_tulip_realm_account() -> Pubkey {
        Pubkey::from_str("413KSeuFUBSWDzfjU9BBqBAWYKmoR8mncrhV84WcGNAk").unwrap()
    }
    pub fn get_tulip_council_mint() -> Pubkey {
        Pubkey::from_str("EzSjCzCPwpchdQVaGJZYpgDNagzasKFVGJ66Dmut26FL").unwrap()
    }
    pub fn get_tulip_community_mint() -> Pubkey {
        Pubkey::from_str("STuLiPmUCUtG1hQcwdc9de9sjYhVsYoucCiWqbApbpM").unwrap()
    }
    pub fn get_tulip_governance_account() -> Pubkey {
        spl_governance::state::governance::get_mint_governance_address(
            &GOVERNANCE_PROGRAM,
            &get_tulip_realm_account(),
            &get_tulip_council_mint(),
        )
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_simple() {
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
            &(0_u32.to_le_bytes()),
        );
        let proposal1_account = rpc.get_account(&proposal1_key).unwrap();
        let mut proposal1_account_tup = (proposal1_key, proposal1_account);
        let proposal1_account_info = proposal1_account_tup.into_account_info();
        let proposal1 = get_proposal_wrapper(&proposal1_account_info).unwrap();

        let proposal2_key = get_proposal_address(
            &GOVERNANCE_PROGRAM,
            &main_gov_key,
            &get_tulip_community_mint(),
            &(1_u32.to_le_bytes()),
        );
        let proposal2_account = rpc.get_account(&proposal2_key).unwrap();
        let mut proposal2_account_tup = (proposal2_key, proposal2_account);
        let proposal2_account_info = proposal2_account_tup.into_account_info();
        let proposal2 = get_proposal_wrapper(&proposal2_account_info).unwrap();

        let opts = tulip_sled_util::config::DbOpts {
            path: "realms_sdk_list_voting.db".to_string(),
            ..Default::default()
        };

        let db = Database::new(opts).unwrap();

        db.insert_realm(&realm).unwrap();
        db.insert_governance(&main_gov).unwrap();
        db.insert_proposal(&proposal1).unwrap();
        db.insert_proposal(&proposal2).unwrap();

        let proposals = db.list_proposals().unwrap();
        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].key, proposal1_key);
        assert_eq!(proposals[1].key, proposal2_key);

        let governances = db.list_governances().unwrap();
        assert_eq!(governances.len(), 1);
        assert_eq!(governances[0].key, main_gov_key);

        let realms = db.list_realms().unwrap();
        assert_eq!(realms.len(), 1);
        assert_eq!(realms[0].key, realm_key);

        std::fs::remove_dir_all("realms_sdk_list_voting.db").unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_populate_database_with_mint() {
        let rpc = RpcClient::new("https://ssc-dao.genesysgo.net".to_string());

        let opts = tulip_sled_util::config::DbOpts {
            path: "realms_sdk_populate_mint.db".to_string(),
            ..Default::default()
        };

        let db = Database::new(opts).unwrap();

        db.populate_database_with_mint_governance(
            get_tulip_realm_account(),
            get_tulip_council_mint(),
            get_tulip_community_mint(),
            Utc::now(),
            &rpc,
        )
        .unwrap();

        let governances = db.list_governances().unwrap();
        assert_eq!(governances.len(), 1);
        let realms = db.list_realms().unwrap();
        assert_eq!(realms.len(), 1);
        let proposals = db.list_proposals().unwrap();
        assert_eq!(
            proposals.len(),
            governances[0].governance.proposals_count as usize
        );

        let notif_cache = db.get_governance_notif_cache(governances[0].key).unwrap();
        assert_eq!(notif_cache.governance_key, governances[0].key);
        assert_eq!(
            notif_cache.last_proposals_count,
            governances[0].governance.proposals_count
        );
        assert_eq!(notif_cache.voting_proposals_last_notification_time.len(), 0);

        std::fs::remove_dir_all("realms_sdk_populate_mint.db").unwrap();
    }
}
