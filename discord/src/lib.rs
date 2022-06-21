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

use chrono::{DateTime, Utc};
use crossbeam::sync::WaitGroup;
use serenity::builder::CreateEmbed;
use serenity::prelude::*;
use spl_token::solana_program::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use config::Configuration;
use crossbeam_channel::select;
use log::{error, info, warn};
use serenity::model::id::{GuildId, UserId};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{event::ResumedEvent, gateway::Ready, id::ChannelId},
    utils::MessageBuilder,
};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct NotifCacheEntry {
    last_notif: DateTime<Utc>,
    // the time at which this entry was last seen. we have this set to an Option
    // as we will not always use this
    last_seen: Option<DateTime<Utc>>,
}

struct Handler {
    is_loop_running: AtomicBool,
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

            tokio::task::spawn(async move {
                loop {
                    select! {
                        recv(exit_chan) -> _msg => {
                            warn!("discord workerloop received exit signal");
                            return;
                        }
                        default() => {
                            // check various matrics
                            if let Err(err) = ChannelId(config.discord.status_channel).send_message(&_ctx, |m | {
                                m.add_embed(|e| {
                                    e.title("🧑‍🌾 Farmer's Almanac - Automated Check In 🧑‍🌾");
                                    e.color((6, 72, 82));
                                    e
                                });
                                m
                            }).await {
                                error!("failed to send status update {:#?}", err);
                            }
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
            is_loop_running: AtomicBool::new(false),
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
