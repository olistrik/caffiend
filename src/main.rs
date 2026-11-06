//! Watches coffee roasters' WooCommerce shops and sends a message when something new appears.

mod config;
mod diff;
mod entities;
mod notify;
mod poll;
mod product;
mod shop;
mod state;

use std::{path::PathBuf, time::Duration};

use anyhow::{Result, bail};
use clap::Parser;

use config::Config;
use notify::{Message, Notifier};
use poll::{Poller, poll_all};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// The JSON configuration file (see the README).
    #[arg(long)]
    config: PathBuf,

    /// Poll every shop once and exit, instead of running until stopped.
    #[arg(long)]
    once: bool,

    /// Send one test message through the configured notifiers and exit.
    #[arg(long)]
    send_test: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    let config = Config::load(&args.config)?;
    let notifier = Notifier::from_config(&config)?;

    if args.send_test {
        return notifier.send(&Message::test()).await;
    }

    std::fs::create_dir_all(&config.state_dir)?;
    let interval = Duration::from_secs(config.interval_secs);
    let pollers = config
        .shops
        .into_iter()
        .map(|(key, shop)| Poller::new(&key, shop, &config.state_dir))
        .collect::<Result<Vec<_>>>()?;

    if args.once {
        return match poll_all(&pollers, &notifier).await {
            0 => Ok(()),
            n => bail!("{n} shop(s) failed"),
        };
    }
    let mut ticker = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = ticker.tick() => { poll_all(&pollers, &notifier).await; }
            _ = tokio::signal::ctrl_c() => return Ok(()),
        }
    }
}
