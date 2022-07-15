//! Requires the 'framework' feature flag be enabled in your project's
//! `Cargo.toml`.
//!
//! This can be enabled by specifying the feature in the dependency section:
//!
//! ```toml
//! [dependencies.serenity]
//! git = "https://github.com/serenity-rs/serenity.git"
//! features = ["framework", "standard_framework"]
//! ```

#![feature(async_closure)]

use chrono::prelude::*;

use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use solana_program::account_info::IntoAccountInfo;
use solana_program::program_pack::Pack;
use std::sync::atomic::AtomicBool;
use std::{collections::HashSet, sync::Arc};
use tulip_realms_sdk::GOVERNANCE_PROGRAM;

use anyhow::Result;
use config::Configuration;
use crossbeam_channel::select;
use log::{error, info, warn};
use serenity::model::id::GuildId;
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{event::ResumedEvent, gateway::Ready, id::ChannelId},
};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[derive(Clone)]
struct Handler {
    is_loop_running: Arc<AtomicBool>,
    config: Arc<Configuration>,
    exit_chan: crossbeam_channel::Receiver<bool>,
}

impl Handler {
    pub fn handle_ready(&self, _ctx: Context) {
        if !self
            .is_loop_running
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            let already_running = self
                .is_loop_running
                .swap(true, std::sync::atomic::Ordering::SeqCst);
            if already_running {
                info!("background task is already running, goodbye");
                return;
            }
            info!("starting background task");
            let sleep_time = self.config.discord.worker_loop_frequency;
            let exit_chan = self.exit_chan.clone();
            let config = self.config.clone();
            let rpc_client = Arc::new(self.config.rpc_client());
            // we need the mint account type used for voting so that we may display vote counts
            // as f64 instead of u64
            let voter_mint = match rpc_client.get_account(&config.realm_info.community_mint_key()) {
                Ok(voter_mint_acct) => {
                    spl_token::state::Mint::unpack_unchecked(&voter_mint_acct.data[..]).unwrap()
                }
                Err(err) => panic!("failed to load community mint {:#?}", err),
            };
            //let handler = Arc::new(self.clone());
            let db = tulip_realms_sdk::Database::new(config.db_opts.clone()).unwrap();
            if let Err(err) = db.sync_notif_cache_with_proposals(
                config.realm_info.realm_key(),
                config.realm_info.council_mint_key(),
                Utc::now(),
                &rpc_client,
            ) {
                log::error!("failed to sync notification cache with proposal {:#?}", err);
            }
            tokio::task::spawn(async move {
                // only send this if debug logs are enabled
                if config.debug_log {
                    let mut msg_builder = MessageBuilder::new();
                    msg_builder.push("listening for new proposals");
                    if let Err(err) = ChannelId(config.discord.status_channel)
                        .say(&_ctx, msg_builder)
                        .await
                    {
                        log::error!("failed to send message {:#?}", err);
                    }
                }
                let do_fn = async || {
                    // check to see if we have any new proposals that were submitted
                    match db.get_governance_notif_cache(config.realm_info.governance_key()) {
                        Ok(mut notif_cache) => {
                            // fetch the governance account
                            let governance_account = {
                                match rpc_client.get_account(&config.realm_info.governance_key()) {
                                    Ok(account) => {
                                        let mut account_tup =
                                            (config.realm_info.governance_key(), account);
                                        let account_info = account_tup.into_account_info();
                                        match tulip_realms_sdk::types::get_governance_wrapper(
                                            &account_info,
                                        ) {
                                            Ok(gov_acct) => gov_acct,
                                            Err(err) => {
                                                log::error!(
                                                    "failed to get governance account {:#?}",
                                                    err
                                                );
                                                return;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("failed to get governance account {:#?}", err);
                                        return;
                                    }
                                }
                            };
                            if governance_account
                                .governance
                                .proposals_count
                                .gt(&notif_cache.last_proposals_count)
                            {
                                let mut new_proposals = Vec::with_capacity(
                                    (governance_account.governance.proposals_count
                                        - notif_cache.last_proposals_count)
                                        as usize,
                                );
                                for idx in notif_cache.last_proposals_count
                                    ..governance_account.governance.proposals_count
                                {
                                    let proposal_key =
                                        spl_governance::state::proposal::get_proposal_address(
                                            &GOVERNANCE_PROGRAM,
                                            &config.realm_info.governance_key(),
                                            &config.realm_info.community_mint_key(),
                                            &idx.to_le_bytes()[..],
                                        );
                                    match rpc_client.get_account(&proposal_key) {
                                        Ok(account) => {
                                            let mut account_tup = (proposal_key, account);
                                            let account_info = account_tup.into_account_info();
                                            match tulip_realms_sdk::types::get_proposal_wrapper(
                                                &account_info,
                                            ) {
                                                Ok(proposal) => {
                                                    new_proposals.push(proposal);
                                                }
                                                Err(err) => {
                                                    log::error!(
                                                        "failed to get proposal account {:#?}",
                                                        err
                                                    );
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            log::error!(
                                                "failed to get proposal account {:#?}",
                                                err
                                            );
                                            continue;
                                        }
                                    }
                                }
                                for proposal in new_proposals.iter() {
                                    if let Err(err) = ChannelId(config.discord.status_channel)
                                        .send_message(&_ctx, |m| {
                                            m.add_embed(|e| {
                                                e.title("New Proposal Detected");
                                                e.field(
                                                    "proposal".to_string(),
                                                    format!(
                                                        "[{}]({}/proposal/{})",
                                                        proposal.key,
                                                        config.discord.ui_base_url,
                                                        proposal.key
                                                    ),
                                                    false,
                                                );
                                                let mut proposal = proposal.proposal.clone();
                                                // truncate description length if longer than 512 chars
                                                proposal.description_link.truncate(
                                                    if proposal.description_link.chars().count()
                                                        > 512
                                                    {
                                                        512_usize
                                                    } else {
                                                        proposal.description_link.len()
                                                    },
                                                );
                                                e.field("name".to_string(), proposal.name, false);
                                                e.field(
                                                    "description",
                                                    proposal.description_link,
                                                    false,
                                                );
                                                e
                                            });
                                            m
                                        })
                                        .await
                                    {
                                        log::error!("failed to send message {:#?}", err);
                                    } else {
                                        let mut contains_proposal = false;
                                        notif_cache
                                            .voting_proposals_last_notification_time
                                            .iter()
                                            .for_each(|(proposal_key, _)| {
                                                if proposal_key.eq(&proposal.key) {
                                                    contains_proposal = true;
                                                }
                                            });
                                        if !contains_proposal {
                                            notif_cache
                                                .voting_proposals_last_notification_time
                                                .push((proposal.key, 0));
                                        }
                                        // only insert proposal after a successful notification
                                        if let Err(err) = db.insert_proposal(proposal) {
                                            log::error!("failed to insert new proposal {:#?}", err);
                                        }
                                    }
                                }
                            }
                            if let Err(err) = db.insert_governance(&governance_account) {
                                log::error!("failed to isnert governance {:#?}", err);
                            }
                            // update the notif cache with the new proposal count
                            notif_cache.last_proposals_count =
                                governance_account.governance.proposals_count;
                            if let Err(err) = db.insert_notif_cache_entry(&notif_cache) {
                                log::error!("failed to insert notif cache {:#?}", err);
                            }
                            // now sync everything
                            if let Err(err) = db.sync_notif_cache_with_proposals(
                                config.realm_info.realm_key(),
                                config.realm_info.council_mint_key(),
                                Utc::now(),
                                &rpc_client,
                            ) {
                                log::error!("failed to sync disk backed cache {:#?}", err);
                            }
                        }
                        Err(err) => {
                            log::error!("failed to load notif cache {:#?}", err);
                        }
                    }
                    if let Err(err) = db.db.flush() {
                        log::error!("failed to flush database {:#?}", err);
                    }
                    // now handle existing proposal notification
                    match db.get_governance_notif_cache(config.realm_info.governance_key()) {
                        Ok(mut notif_cache) => {
                            // fetch the governance account
                            let governance_account = {
                                match rpc_client.get_account(&config.realm_info.governance_key()) {
                                    Ok(account) => {
                                        let mut account_tup =
                                            (config.realm_info.governance_key(), account);
                                        let account_info = account_tup.into_account_info();
                                        match tulip_realms_sdk::types::get_governance_wrapper(
                                            &account_info,
                                        ) {
                                            Ok(gov_acct) => gov_acct,
                                            Err(err) => {
                                                log::error!(
                                                    "failed to get governance account {:#?}",
                                                    err
                                                );
                                                return;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("failed to get governance account {:#?}", err);
                                        return;
                                    }
                                }
                            };
                            log::info!("notif cache\n{:#?}", notif_cache);
                            let mut finished_proposals = Vec::with_capacity(
                                notif_cache.voting_proposals_last_notification_time.len(),
                            );
                            for (proposal_key, last_notif_time) in notif_cache
                                .voting_proposals_last_notification_time
                                .iter_mut()
                            {
                                let now = Utc::now();
                                let last_notif_ts =
                                    tulip_realms_sdk::utils::date_time_from_timestamp(
                                        *last_notif_time,
                                    );
                                match db.get_proposal(*proposal_key) {
                                    Ok(proposal) => {
                                        if !proposal.has_vote_time_ended(
                                            &governance_account.governance.config,
                                            now,
                                        ) && now.gt(&last_notif_ts)
                                        {
                                            let duration_diff =
                                                now.signed_duration_since(last_notif_ts);
                                            if duration_diff.ge(&chrono::Duration::hours(
                                                config.discord.notification_frequency,
                                            )) {
                                                if let Some(ends_at) = proposal.vote_ends_at(
                                                    &governance_account.governance.config,
                                                ) {
                                                    let time_until_end =
                                                        ends_at.signed_duration_since(now);
                                                    let voter_records = match tulip_realms_sdk::utils::get_vote_records_for_proposal(
                                                        &rpc_client,
                                                        proposal.key,
                                                    ) {
                                                        Ok(voter_records) => voter_records,
                                                        Err(err) => {
                                                            log::error!("failed to fetch voter records for proposal {}: {:#?}", proposal.key, err);
                                                            vec![]
                                                        }
                                                    };
                                                    let mut approval_votes = 0;
                                                    let mut deny_votes = 0;
                                                    // do not track relinquished votes
                                                    for voter_record in
                                                        voter_records.iter().filter(|vote_record| {
                                                            !vote_record.is_relinquished
                                                        })
                                                    {
                                                        match voter_record.vote {
                                                            spl_governance::state::vote_record::Vote::Approve(_) => {
                                                                approval_votes += voter_record.voter_weight
                                                            }
                                                            spl_governance::state::vote_record::Vote::Deny => {
                                                                deny_votes += voter_record.voter_weight
                                                            }
                                                            _ => log::warn!("unsupported vote type {:#?}", voter_record.vote)
                                                        }
                                                    }
                                                    let approval_votes = if approval_votes == 0 {
                                                        0.0
                                                    } else {
                                                        spl_token::amount_to_ui_amount(
                                                            approval_votes,
                                                            voter_mint.decimals,
                                                        )
                                                    };
                                                    let deny_votes = if deny_votes == 0 {
                                                        0.0
                                                    } else {
                                                        spl_token::amount_to_ui_amount(
                                                            deny_votes,
                                                            voter_mint.decimals,
                                                        )
                                                    };
                                                    if let Err(err) = ChannelId(config.discord.status_channel)
                                                        .send_message(&_ctx, |m| {
                                                            m.add_embed(|e| {
                                                                e.title("Proposal Voting Stats".to_string());
                                                                e.description("stats for proposals accepting votes".to_string());
                                                                e.field(
                                                                    "proposal".to_string(), 
                                                                    format!("[{}]({}/proposal/{})", proposal.key, config.discord.ui_base_url, proposal.key),
                                                                    false,
                                                                );
                                                                let mut proposal = proposal.proposal.clone();
                                                                // truncate description length if longer than 512 chars
                                                                proposal.description_link.truncate(
                                                                    if proposal.description_link.chars().count()
                                                                        > 512
                                                                    {
                                                                        512_usize
                                                                    } else {
                                                                        proposal.description_link.len()
                                                                    },
                                                                );
                                                                e.field("name".to_string(), proposal.name, false);
                                                                let description = if proposal.description_link.eq_ignore_ascii_case("") {
                                                                    "no description provided".to_string()
                                                                } else {
                                                                    proposal.description_link.clone()
                                                                };
                                                                e.field(
                                                                    "description",
                                                                    description.as_str(),
                                                                    false,
                                                                );
                                                                e.field(
                                                                    "approval vote count",
                                                                    approval_votes.to_string().as_str(),
                                                                    false,
                                                                );
                                                                e.field(
                                                                    "deny vote count",
                                                                    deny_votes.to_string().as_str(),
                                                                    false,
                                                                );
                                                                e.field(
                                                                    "time left".to_string(),
                                                                    format!("{} hours", time_until_end.num_hours()),
                                                                     false,
                                                                );
                                                                log::info!("embed {:#?}", e);
                                                                e
                                                            });
                                                            m
                                                        })
                                                        .await
                                                        {
                                                            log::error!("failed to send message {:#?}", err);
                                                        } else {
                                                            *last_notif_time = now.timestamp();
                                                        }
                                                }
                                            }
                                        }
                                        // mark a proposal as finished if vote time has ended **or** state is not voting
                                        let inserted = if proposal.has_vote_time_ended(
                                            &governance_account.governance.config,
                                            now,
                                        ) {
                                            finished_proposals.push(proposal.key);
                                            true
                                        } else {
                                            false
                                        };
                                        if !inserted && proposal.proposal.state.ne(
                                            &spl_governance::state::enums::ProposalState::Voting,
                                        ) {
                                            finished_proposals.push(proposal.key);
                                        }
                                        log::info!(
                                            "proposal {}, state {:#?}",
                                            proposal.key,
                                            proposal.proposal.state
                                        );
                                    }
                                    Err(err) => {
                                        log::error!(
                                            "failed to get proposal for {}: {:#?}",
                                            err,
                                            proposal_key
                                        );
                                    }
                                }
                            }
                            log::info!("checking for proposals to remove");
                            // remove any proposals which finished
                            for proposal in finished_proposals.iter() {
                                log::info!("checking proposal {}", proposal);
                                if let Ok(prop_info) = db.get_proposal(*proposal) {
                                    log::info!(
                                        "checking proposal {}, state {:#?}",
                                        proposal,
                                        prop_info.proposal.state
                                    );
                                } else {
                                    continue;
                                }
                                for (idx, (key, _)) in notif_cache
                                    .clone()
                                    .voting_proposals_last_notification_time
                                    .iter()
                                    .enumerate()
                                {
                                    if proposal.eq(key) {
                                        log::info!("removing proposal {}", proposal);
                                        // remove this index
                                        notif_cache
                                            .voting_proposals_last_notification_time
                                            .swap_remove(idx);
                                        break;
                                    }
                                }
                            }
                            if let Err(err) = db.insert_notif_cache_entry(&notif_cache) {
                                log::error!("failed to update notification cache {:#?}", err);
                            }
                            if let Err(err) = db.insert_governance(&governance_account) {
                                log::error!("failed to update governance account {:#?}", err);
                            }
                            if let Err(err) = db.db.flush() {
                                log::error!("failed to flush database {:#?}", err);
                            }
                        }

                        Err(err) => {
                            log::error!("failed to load notif cache {:#?}", err);
                        }
                    }
                };
                loop {
                    select! {
                        recv(exit_chan) -> _msg => {
                            warn!("discord workerloop received exit signal");
                            return;
                        }
                        default() => {
                            do_fn().await;
                            std::thread::sleep(std::time::Duration::from_secs(sleep_time));
                        }
                    }
                }
            });
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    // use this to spawn a task to log messages
    async fn ready(&self, ctx: Context, _ready: Ready) {
        info!("Connected as {}", _ready.user.name);
        self.handle_ready(ctx);
    }
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        self.handle_ready(ctx);
    }
    async fn resume(&self, ctx: Context, _: ResumedEvent) {
        self.handle_ready(ctx);
        info!("Resumed");
    }
}

#[group]
struct General;

pub async fn start_discord_bot(
    config: &Arc<config::Configuration>,
    exit_chan: crossbeam_channel::Receiver<bool>,
) -> Result<()> {
    info!("starting bot");

    let http = Http::new(&config.discord.bot_token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let mut broadcaster = channels::broadcast::UnboundedBroadcast::new();
    let subscriber = broadcaster.subscribe();
    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix("~")
                .allow_dm(false)
                .ignore_bots(true)
                .allowed_channels(
                    vec![ChannelId(config.discord.status_channel)]
                        .into_iter()
                        .collect(),
                )
                .with_whitespace(true)
                .on_mention(Some(serenity::model::id::UserId(_bot_id.0)))
                .owners(owners)
        })
        .group(&GENERAL_GROUP);

    // create the intents
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // initialize the framework, and event handler
    let mut client = Client::builder(&config.discord.bot_token, intents)
        .event_handler(Handler {
            is_loop_running: Arc::new(AtomicBool::new(false)),
            config: Arc::clone(config),
            exit_chan: subscriber,
        })
        .framework(framework)
        .await?;
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        select! {
            recv(exit_chan) -> _msg => {
                warn!("received exit signal");
                // todo(bonedaddy): should we add a waitgroup here
                if let Err(err) = broadcaster.send(true) {
                    error!("discord bot failed to notify workers to exit {:#?}", err);
                }
                // hacky workaround to give worker loops time to exit
                // definitely needs to have some better thread synchronization
                std::thread::sleep(std::time::Duration::from_secs(5));
                shard_manager.lock().await.shutdown_all().await;
                info!("shutdown finalized, goodbye...")
            }
        }
    });
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
