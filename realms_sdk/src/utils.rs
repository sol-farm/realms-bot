use chrono::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_filter::RpcFilterType;
use solana_program::pubkey::Pubkey;
use spl_governance::state::vote_record::VoteRecordV2;

use crate::GOVERNANCE_TREE;
use crate::{
    types::{GovernanceV2Wrapper, ProposalV2Wrapper},
    Database,
};
use anyhow::Result;
use tulip_sled_util::types::DbTrees;
impl Database {
    /// returns a vector of all proposals that are undergoing activte voting
    pub fn list_voting_proposals(&self, now: DateTime<Utc>) -> Result<Vec<ProposalV2Wrapper>> {
        let mut governance_wrapper = None;
        let voting_proposals = self
            .list_proposals()?
            .into_iter()
            .filter(|proposal| {
                if proposal.proposal.voting_at.is_none() {
                    return false;
                }
                if governance_wrapper.is_none() {
                    if let Ok(gov_tree) = self.db.open_tree(DbTrees::Custom(GOVERNANCE_TREE)) {
                        let gov_wrapper: GovernanceV2Wrapper = if let Ok(gov_wrap) =
                            gov_tree.deserialize(proposal.proposal.governance)
                        {
                            gov_wrap
                        } else {
                            log::warn!("failed to deserialize governance account");
                            return false;
                        };
                        governance_wrapper = Some(gov_wrapper);
                    }
                }
                if let Some(governance_wrapper) = governance_wrapper.as_ref() {
                    //return !proposal.has_vote_time_ended(&governance_wrapper.governance.config, now);
                    !proposal.has_vote_time_ended(&governance_wrapper.governance.config, now)
                } else {
                    log::warn!("governance wrapper is None");
                    false
                }
            })
            .collect();
        Ok(voting_proposals)
    }
}

/// given a timestamp, return a DateTime<Utc> object using a utc timezone
pub fn date_time_from_timestamp(timestamp: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc)
}

pub fn governance_notif_cache_key(gov_key: Pubkey) -> String {
    format!("notif_cache_entry-{}", gov_key)
}



pub fn get_vote_records_for_proposal(
    rpc: &RpcClient,
    proposal: Pubkey,
) -> Result<()> {
    use crate::GOVERNANCE_PROGRAM;
    use solana_client::rpc_config::RpcProgramAccountsConfig;
    use solana_client::rpc_filter::Memcmp;
    use solana_client::rpc_config::RpcAccountInfoConfig;
    use solana_account_decoder::UiAccountEncoding;
    match rpc.get_program_accounts_with_config(
        &GOVERNANCE_PROGRAM,
        RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::DataSize(std::mem::size_of::<spl_governance::state::vote_record::VoteRecordV2>() as u64),
                RpcFilterType::Memcmp(Memcmp {
                    // -1 because the account data buffer is an array
                    offset: std::mem::size_of::<spl_governance::state::enums::GovernanceAccountType>() - 1,
                    bytes: solana_client::rpc_filter::MemcmpEncodedBytes::Bytes(
                        proposal.to_bytes().to_vec(),
                    ),
                    encoding: None,
                }),
            ]),
            with_context: None,
            account_config: RpcAccountInfoConfig {
                min_context_slot: None,
                encoding: Some(UiAccountEncoding::Base64),
                data_slice: None,
                commitment: None,
            },
        }
    ) {
        Ok(accounts) => {
            println!("found {} vote records", accounts.len());
        }
        Err(err) => {
            log::error!("failed to vote account records {:#?}", err);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::{get_tulip_community_mint, get_tulip_council_mint, get_tulip_realm_account};
    use solana_client::rpc_client::RpcClient;
    use static_pubkey::static_pubkey;
    #[tokio::test(flavor = "multi_thread")]

    async fn test_get_vote_records_for_proposal() {
        let proposal = static_pubkey!("9z4TmXcvSUksTB1LiUSHYFxoodH67Fi2Wt5riCo7i61U");
        let rpc = RpcClient::new("http://51.222.241.93:8899".to_string());
        get_vote_records_for_proposal(&rpc, proposal).unwrap();
    }
    #[test]
    fn test_timestamp() {
        let now = Utc::now();
        let now_ts = now.timestamp();
        let got_now = date_time_from_timestamp(now_ts);
        assert_eq!(now.year(), got_now.year());
        assert_eq!(now.day(), got_now.day());
        assert_eq!(now.hour(), got_now.hour());
        assert_eq!(now.minute(), got_now.minute());
        // note: for some reason the sec/ns dont seem to always align
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_voting_proposals() {
        let rpc = RpcClient::new("https://ssc-dao.genesysgo.net".to_string());

        let opts = tulip_sled_util::config::DbOpts {
            path: "realms_sdk_list_voting2.db".to_string(),
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

        // because this test fetches data at run time, use a fixed point in time as "now"
        // htis is roughly tue jun 21st 11am EST
        let now = date_time_from_timestamp(1655842130);

        // because this fetches information at runtime, we may not always have a proposal to vote on
        let voting_proposals = db.list_voting_proposals(now).unwrap();
        assert_eq!(voting_proposals.len(), 0);

        let now = now.checked_sub_signed(chrono::Duration::days(60)).unwrap();
        let voting_proposals = db.list_voting_proposals(now).unwrap();
        assert_eq!(voting_proposals.len(), 6);

        std::fs::remove_dir_all("realms_sdk_list_voting2.db").unwrap();
    }
}
