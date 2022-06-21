//! Program instructions

use crate::state::{
    enums::MintMaxVoteWeightSource,
    governance::{
        get_account_governance_address, get_mint_governance_address,
        get_program_governance_address, get_token_governance_address, GovernanceConfig,
    },
    proposal::get_proposal_address,
    proposal_instruction::{get_proposal_instruction_address, InstructionData},
    realm::{get_governing_token_holding_address, get_realm_address, RealmConfigArgs},
    signatory_record::get_signatory_record_address,
    token_owner_record::get_token_owner_record_address,
    vote_record::get_vote_record_address,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    bpf_loader_upgradeable,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Yes/No Vote
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum Vote {
    /// Yes vote
    Yes,
    /// No vote
    No,
}

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[repr(C)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance Realm account which aggregates governances for given Community Mint and optional Council Mint
    ///
    /// 0. `[writable]` Governance Realm account. PDA seeds:['governance',name]
    /// 1. `[]` Realm authority
    /// 2. `[]` Community Token Mint
    /// 3. `[writable]` Community Token Holding account. PDA seeds: ['governance',realm,community_mint]
    ///     The account will be created with the Realm PDA as its owner
    /// 4. `[signer]` Payer
    /// 5. `[]` System
    /// 6. `[]` SPL Token
    /// 7. `[]` Sysvar Rent
    /// 8. `[]` Council Token Mint - optional
    /// 9. `[writable]` Council Token Holding account - optional unless council is used. PDA seeds: ['governance',realm,council_mint]
    ///     The account will be created with the Realm PDA as its owner
    CreateRealm {
        #[allow(dead_code)]
        /// UTF-8 encoded Governance Realm name
        name: String,

        #[allow(dead_code)]
        /// Realm config args     
        config_args: RealmConfigArgs,
    },

    /// Deposits governing tokens (Community or Council) to Governance Realm and establishes your voter weight to be used for voting within the Realm
    /// Note: If subsequent (top up) deposit is made and there are active votes for the Voter then the vote weights won't be updated automatically
    /// It can be done by relinquishing votes on active Proposals and voting again with the new weight
    ///
    ///  0. `[]` Governance Realm account
    ///  1. `[writable]` Governing Token Holding account. PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Source account. All tokens from the account will be transferred to the Holding account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[signer]` Governing Token Transfer authority
    ///  5. `[writable]` Token Owner Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///  6. `[signer]` Payer
    ///  7. `[]` System
    ///  8. `[]` SPL Token
    ///  9. `[]` Sysvar Rent
    DepositGoverningTokens {},

    /// Withdraws governing tokens (Community or Council) from Governance Realm and downgrades your voter weight within the Realm
    /// Note: It's only possible to withdraw tokens if the Voter doesn't have any outstanding active votes
    /// If there are any outstanding votes then they must be relinquished before tokens could be withdrawn
    ///
    ///  0. `[]` Governance Realm account
    ///  1. `[writable]` Governing Token Holding account. PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Destination account. All tokens will be transferred to this account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[writable]` Token Owner  Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///  5. `[]` SPL Token
    WithdrawGoverningTokens {},

    /// Sets Governance Delegate for the given Realm and Governing Token Mint (Community or Council)
    /// The Delegate would have voting rights and could vote on behalf of the Governing Token Owner
    /// The Delegate would also be able to create Proposals on behalf of the Governing Token Owner
    /// Note: This doesn't take voting rights from the Token Owner who still can vote and change governance_delegate
    ///
    /// 0. `[signer]` Current Governance Delegate or Governing Token owner
    /// 1. `[writable]` Token Owner  Record
    SetGovernanceDelegate {
        #[allow(dead_code)]
        /// New Governance Delegate
        new_governance_delegate: Option<Pubkey>,
    },

    /// Creates Account Governance account which can be used to govern an arbitrary account
    ///
    ///   0. `[]` Realm account the created Governance belongs to
    ///   1. `[writable]` Account Governance account. PDA seeds: ['account-governance', realm, governed_account]
    ///   2. `[]` Account governed by this Governance
    ///   3. `[]` Governing TokenOwnerRecord account
    ///   4. `[signer]` Payer
    ///   5. `[]` System program
    ///   6. `[]` Sysvar Rent
    CreateAccountGovernance {
        /// Governance config
        #[allow(dead_code)]
        config: GovernanceConfig,
    },

    /// Creates Program Governance account which governs an upgradable program
    ///
    ///   0. `[]` Realm account the created Governance belongs to
    ///   1. `[writable]` Program Governance account. PDA seeds: ['program-governance', realm, governed_program]
    ///   2. `[]` Program governed by this Governance account
    ///   3. `[writable]` Program Data account of the Program governed by this Governance account
    ///   4. `[signer]` Current Upgrade Authority account of the Program governed by this Governance account
    ///   5. `[]` Governing TokenOwnerRecord account     
    ///   6. `[signer]` Payer
    ///   7. `[]` bpf_upgradeable_loader program
    ///   8. `[]` System program
    ///   9. `[]` Sysvar Rent
    CreateProgramGovernance {
        /// Governance config
        #[allow(dead_code)]
        config: GovernanceConfig,

        #[allow(dead_code)]
        /// Indicates whether Program's upgrade_authority should be transferred to the Governance PDA
        /// If it's set to false then it can be done at a later time
        /// However the instruction would validate the current upgrade_authority signed the transaction nonetheless
        transfer_upgrade_authority: bool,
    },

    /// Creates Proposal account for Instructions that will be executed at some point in the future
    ///
    ///   0. `[]` Realm account the created Proposal belongs to
    ///   1. `[writable]` Proposal account. PDA seeds ['governance',governance, governing_token_mint, proposal_index]
    ///   2. `[writable]` Governance account
    ///   3. `[writable]` TokenOwnerRecord account of the Proposal owner
    ///   4. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Rent sysvar
    ///   8. `[]` Clock sysvar
    CreateProposal {
        #[allow(dead_code)]
        /// UTF-8 encoded name of the proposal
        name: String,

        #[allow(dead_code)]
        /// Link to gist explaining proposal
        description_link: String,

        #[allow(dead_code)]
        /// Governing Token Mint the Proposal is created for
        governing_token_mint: Pubkey,
    },

    /// Adds a signatory to the Proposal which means this Proposal can't leave Draft state until yet another Signatory signs
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[writable]` Signatory Record Account
    ///   4. `[signer]` Payer
    ///   5. `[]` System program
    ///   6. `[]` Rent sysvar
    AddSignatory {
        #[allow(dead_code)]
        /// Signatory to add to the Proposal
        signatory: Pubkey,
    },

    /// Removes a Signatory from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[writable]` Signatory Record Account
    ///   4. `[writable]` Beneficiary Account which would receive lamports from the disposed Signatory Record Account
    RemoveSignatory {
        #[allow(dead_code)]
        /// Signatory to remove from the Proposal
        signatory: Pubkey,
    },

    /// Inserts an instruction for the Proposal at the given index position
    /// New Instructions must be inserted at the end of the range indicated by Proposal instructions_next_index
    /// If an Instruction replaces an existing Instruction at a given index then the old one must be removed using RemoveInstruction first

    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[]` TokenOwnerRecord account of the Proposal owner
    ///   3. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   4. `[writable]` ProposalInstruction account. PDA seeds: ['governance',proposal,index]
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Clock sysvar
    InsertInstruction {
        #[allow(dead_code)]
        /// Instruction index to be inserted at.
        index: u16,
        #[allow(dead_code)]
        /// Waiting time (in seconds) between vote period ending and this being eligible for execution
        hold_up_time: u32,

        #[allow(dead_code)]
        /// Instruction Data
        instruction: InstructionData,
    },

    /// Removes instruction from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[writable]` ProposalInstruction account
    ///   4. `[writable]` Beneficiary Account which would receive lamports from the disposed ProposalInstruction account
    RemoveInstruction,

    /// Cancels Proposal by changing its state to Canceled
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]`  TokenOwnerRecord account of the  Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[]` Clock sysvar
    CancelProposal,

    /// Signs off Proposal indicating the Signatory approves the Proposal
    /// When the last Signatory signs the Proposal state moves to Voting state
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Signatory Record account
    ///   2. `[signer]` Signatory account
    ///   3. `[]` Clock sysvar
    SignOffProposal,

    ///  Uses your voter weight (deposited Community or Council tokens) to cast a vote on a Proposal
    ///  By doing so you indicate you approve or disapprove of running the Proposal set of instructions
    ///  If you tip the consensus then the instructions can begin to be run after their hold up time
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   4. `[writable]` TokenOwnerRecord of the Proposal owner    
    ///   3. `[writable]` TokenOwnerRecord of the voter. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   4. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   5. `[writable]` Proposal VoteRecord account. PDA seeds: ['governance',proposal,governing_token_owner_record]
    ///   6. `[]` Governing Token Mint
    ///   7. `[signer]` Payer
    ///   8. `[]` System program
    ///   9. `[]` Rent sysvar
    ///   10. `[]` Clock sysvar
    CastVote {
        #[allow(dead_code)]
        /// Yes/No vote
        vote: Vote,
    },

    /// Finalizes vote in case the Vote was not automatically tipped within max_voting_time period
    ///
    ///   0. `[]` Realm account    
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord of the Proposal owner        
    ///   4. `[]` Governing Token Mint
    ///   5. `[]` Clock sysvar
    FinalizeVote {},

    ///  Relinquish Vote removes voter weight from a Proposal and removes it from voter's active votes
    ///  If the Proposal is still being voted on then the voter's weight won't count towards the vote outcome
    ///  If the Proposal is already in decided state then the instruction has no impact on the Proposal
    ///  and only allows voters to prune their outstanding votes in case they wanted to withdraw Governing tokens from the Realm
    ///
    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[writable]` TokenOwnerRecord account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   3. `[writable]` Proposal VoteRecord account. PDA seeds: ['governance',proposal,governing_token_owner_record]
    ///   4. `[]` Governing Token Mint
    ///   5. `[signer]` Optional Governance Authority (Token Owner or Governance Delegate)
    ///       It's required only when Proposal is still being voted on
    ///   6. `[writable]` Optional Beneficiary account which would receive lamports when VoteRecord Account is disposed
    ///       It's required only when Proposal is still being voted on
    RelinquishVote,

    /// Executes an instruction in the Proposal
    /// Anybody can execute transaction once Proposal has been voted Yes and transaction_hold_up time has passed
    /// The actual instruction being executed will be signed by Governance PDA the Proposal belongs to
    /// For example to execute Program upgrade the ProgramGovernance PDA would be used as the singer
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` ProposalInstruction account you wish to execute
    ///   2. `[]` Clock sysvar
    ///   3+ Any extra accounts that are part of the instruction, in order
    ExecuteInstruction,

    /// Creates Mint Governance account which governs a mint
    ///
    ///   0. `[]` Realm account the created Governance belongs to    
    ///   1. `[writable]` Mint Governance account. PDA seeds: ['mint-governance', realm, governed_mint]
    ///   2. `[writable]` Mint governed by this Governance account
    ///   3. `[signer]` Current Mint Authority
    ///   4. `[]` Governing TokenOwnerRecord account    
    ///   5. `[signer]` Payer
    ///   6. `[]` SPL Token program
    ///   7. `[]` System program
    ///   8. `[]` Sysvar Rent
    CreateMintGovernance {
        #[allow(dead_code)]
        /// Governance config
        config: GovernanceConfig,

        #[allow(dead_code)]
        /// Indicates whether Mint's authority should be transferred to the Governance PDA
        /// If it's set to false then it can be done at a later time
        /// However the instruction would validate the current mint authority signed the transaction nonetheless
        transfer_mint_authority: bool,
    },

    /// Creates Token Governance account which governs a token account
    ///
    ///   0. `[]` Realm account the created Governance belongs to    
    ///   1. `[writable]` Token Governance account. PDA seeds: ['token-governance', realm, governed_token]
    ///   2. `[writable]` Token account governed by this Governance account
    ///   3. `[signer]` Current Token account
    ///   4. `[]` Governing TokenOwnerRecord account        
    ///   5. `[signer]` Payer
    ///   6. `[]` SPL Token program
    ///   7. `[]` System program
    ///   8. `[]` Sysvar Rent
    CreateTokenGovernance {
        #[allow(dead_code)]
        /// Governance config
        config: GovernanceConfig,

        #[allow(dead_code)]
        /// Indicates whether token owner should be transferred to the Governance PDA
        /// If it's set to false then it can be done at a later time
        /// However the instruction would validate the current token owner signed the transaction nonetheless
        transfer_token_owner: bool,
    },

    /// Sets GovernanceConfig for a Governance
    ///
    ///   0. `[]` Realm account the Governance account belongs to    
    ///   1. `[writable, signer]` The Governance account the config is for
    SetGovernanceConfig {
        #[allow(dead_code)]
        /// New governance config
        config: GovernanceConfig,
    },

    /// Flags an instruction and its parent Proposal with error status
    /// It can be used by Proposal owner in case the instruction is permanently broken and can't be executed
    /// Note: This instruction is a workaround because currently it's not possible to catch errors from CPI calls
    ///       and the Governance program has no way to know when instruction failed and flag it automatically
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)    
    ///   3. `[writable]` ProposalInstruction account to flag
    ///   4. `[]` Clock sysvar
    FlagInstructionError,

    /// Sets new Realm authority
    ///
    ///   0. `[writable]` Realm account
    ///   1. `[signer]` Current Realm authority    
    SetRealmAuthority {
        #[allow(dead_code)]
        /// New realm authority or None to remove authority
        new_realm_authority: Option<Pubkey>,
    },

    /// Sets realm config
    ///   0. `[writable]` Realm account
    ///   1. `[signer]`  Realm authority    
    ///   2. `[]` Council Token Mint - optional
    ///       Note: In the current version it's only possible to remove council mint (set it to None)
    ///       After setting council to None it won't be possible to withdraw the tokens from the Realm any longer
    ///       If that's required then it must be done before executing this instruction
    ///   3. `[writable]` Council Token Holding account - optional unless council is used. PDA seeds: ['governance',realm,council_mint]
    ///       The account will be created with the Realm PDA as its owner
    SetRealmConfig {
        #[allow(dead_code)]
        /// Realm config args
        config_args: RealmConfigArgs,
    },
}
