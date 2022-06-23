use chrono::prelude::*;
use spl_governance::state::governance::GovernanceConfig;

use crate::utils::governance_notif_cache_key;

use super::*;

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct NotifCacheEntry {
    pub governance_key: Pubkey,
    /// the total number of proposals tracked by the governance account the last time
    /// a sample was taken
    pub last_proposals_count: u32,
    /// a vector at which the time a proposal which is actively voting
    /// had a notification sent out, each element contains the values of (proposal_key, notif_time)
    ///
    /// if notif_time is 0, then it means no notification was sent out
    pub voting_proposals_last_notification_time: Vec<(Pubkey, i64)>,
}

impl DbKey for NotifCacheEntry {
    fn key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(governance_notif_cache_key(self.governance_key)
            .as_bytes()
            .to_vec())
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
pub fn get_realm_wrapper(realm_account: &AccountInfo) -> Result<RealmV2Wrapper> {
    let realm_data =
        spl_governance::state::realm::get_realm_data(&GOVERNANCE_PROGRAM, realm_account)?;
    Ok(RealmV2Wrapper {
        realm: realm_data,
        key: *realm_account.key,
    })
}

/// returns a ProposalV2Wrapper if the account can be deserialized into a ProposalV2 account
pub fn get_proposal_wrapper(proposal_account: &AccountInfo) -> Result<ProposalV2Wrapper> {
    let prop_data =
        spl_governance::state::proposal::get_proposal_data(&GOVERNANCE_PROGRAM, proposal_account)?;
    Ok(ProposalV2Wrapper {
        proposal: prop_data,
        key: *proposal_account.key,
    })
}

/// returns a GovernanceV2Wrapper if the account can be deserialized into a ProposalV2 account
pub fn get_governance_wrapper(governance_account: &AccountInfo) -> Result<GovernanceV2Wrapper> {
    let gov_data = spl_governance::state::governance::get_governance_data(
        &GOVERNANCE_PROGRAM,
        governance_account,
    )?;
    Ok(GovernanceV2Wrapper {
        governance: gov_data,
        key: *governance_account.key,
    })
}

impl ProposalV2Wrapper {
    /// similar to ProposalV2::has_vote_time_ended except makes comparisons using timestampts coerced
    /// to utc timezone
    pub fn has_vote_time_ended(
        &self,
        governance_config: &GovernanceConfig,
        now: DateTime<Utc>,
    ) -> bool {
        if let Some(voting_at) = self.proposal.voting_at {
            let voting_at = crate::utils::date_time_from_timestamp(voting_at);
            if let Some(voting_ends_at) = voting_at.checked_add_signed(chrono::Duration::seconds(
                governance_config.max_voting_time as i64,
            )) {
                now.gt(&voting_ends_at)
            } else {
                false
            }
        } else {
            false
        }
    }
    /// this is a very basic version of ProposalV2::finalize_vote and simply sets `voting_compled_at` if the current
    /// timestamp is past the end at time.
    ///
    /// using this as a temporary workaround for `max_voter_weight` as im not entirely sure what its used for. this also
    /// functions slightly differently than ProposalV2::finalized_vote and sets the voting_completed_at time, to the time
    /// at which voting would complete at, not the time at which the vote is finalized
    pub fn finalize_vote(&mut self, governance_config: &GovernanceConfig, now: DateTime<Utc>) {
        if self
            .proposal
            .assert_can_finalize_vote(governance_config, now.timestamp())
            .is_ok()
        {
            if let Some(voting_at) = self.proposal.voting_at {
                self.proposal.voting_completed_at = if let Some(ends_at) =
                    crate::utils::date_time_from_timestamp(voting_at).checked_add_signed(
                        chrono::Duration::seconds(governance_config.max_voting_time as i64),
                    ) {
                    Some(ends_at.timestamp())
                } else {
                    return;
                }
            }
        }
    }
    pub fn vote_ends_at(&self, governance_config: &GovernanceConfig) -> Option<DateTime<Utc>> {
        if let Some(voting_at) = self.proposal.voting_at {
            crate::utils::date_time_from_timestamp(voting_at).checked_add_signed(
                chrono::Duration::seconds(governance_config.max_voting_time as i64),
            )
        } else {
            None
        }
    }
}
