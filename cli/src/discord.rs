use anyhow::Result;
use log::error;
use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
pub async fn start<'a>(_matches: &clap::ArgMatches<'a>, config_file_path: String) -> Result<()> {
    let config = config::Configuration::load(&config_file_path, false)?;
    config.init_log(false);
    let mut broadcaster = channels::broadcast::UnboundedBroadcast::new();
    let subscriber = broadcaster.subscribe();
    let mut signals =
        Signals::new(vec![SIGINT, SIGTERM, SIGQUIT]).expect("failed to registers signals");
    {
        tokio::task::spawn_blocking(move || {
            if let Some(sig) = signals.forever().next() {
                error!("caught signal {:#?}", sig);
            }
            if let Err(err) = broadcaster.send(true) {
                error!("broadcaster failed to notify {:#?}", err);
            }
        });
    }

    discord::start_discord_bot(&Arc::new(config), subscriber).await?;

    Ok(())
}
