//! Watches a coffee roaster's shop and sends a message when a coffee is new or back in stock.

mod diff;
mod entities;
mod notify;
mod poll;
mod product;
mod shop;
mod state;

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;

use notify::{Notifier, read_secret};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// The shop's address; its WooCommerce Store API is read from there.
    #[arg(long, default_value = "https://www.dakcoffeeroasters.com")]
    shop_url: String,

    /// Only announce products in these category slugs (repeatable). Empty means every product.
    #[arg(long = "category", default_values_t = ["filter".to_owned(), "espresso".to_owned()])]
    categories: Vec<String>,

    /// Also announce coffees that were sold out and are back in stock. Off by default.
    #[arg(long)]
    restocks: bool,

    /// Seconds between polls.
    #[arg(long, default_value_t = 600)]
    interval_secs: u64,

    /// Directory for the remembered products.
    #[arg(long, default_value = ".")]
    state_dir: PathBuf,

    /// Poll once and exit, instead of running until stopped.
    #[arg(long)]
    once: bool,

    /// Send one test message through the configured notifiers and exit.
    #[arg(long)]
    send_test: bool,

    /// ntfy server to publish to; needs `--ntfy-topic-file`.
    #[arg(long, default_value = "https://ntfy.sh")]
    ntfy_server: String,

    /// File holding the ntfy topic. The topic is the only secret on a public server.
    #[arg(long)]
    ntfy_topic_file: Option<PathBuf>,

    /// File holding a Telegram bot token; needs `--telegram-chat-id`.
    #[arg(long, requires = "telegram_chat_id")]
    telegram_token_file: Option<PathBuf>,

    #[arg(long, requires = "telegram_token_file")]
    telegram_chat_id: Option<String>,
}

fn notifiers(args: &Args) -> Result<Vec<Notifier>> {
    let mut all = Vec::new();
    if let Some(file) = &args.ntfy_topic_file {
        all.push(Notifier::Ntfy {
            client: reqwest::Client::new(),
            server: args.ntfy_server.clone(),
            topic: read_secret(file)?,
        });
    }
    if let (Some(file), Some(chat_id)) = (&args.telegram_token_file, &args.telegram_chat_id) {
        all.push(Notifier::Telegram {
            client: reqwest::Client::new(),
            token: read_secret(file)?,
            chat_id: chat_id.clone(),
        });
    }
    if all.is_empty() {
        tracing::warn!("no notifier configured: messages are only logged");
        all.push(Notifier::Log);
    }
    Ok(all)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    std::fs::create_dir_all(&args.state_dir)
        .with_context(|| format!("creating {}", args.state_dir.display()))?;
    let state_path = args.state_dir.join("state.json");
    let shop = shop::Shop::new(&args.shop_url)?;
    let notifier = notify::Notifier::Many(notifiers(&args)?);
    if args.send_test {
        let message = notify::Message {
            title: "caffiend test".to_owned(),
            body: "If you can read this, notifications work.".to_owned(),
            url: args.shop_url.clone(),
        };
        return notifier.send(&message).await;
    }
    let poller = poll::Poller {
        shop: &shop,
        notifier: &notifier,
        state_path: &state_path,
        categories: &args.categories,
        restocks: args.restocks,
    };

    if args.once {
        return poller.run_once().await;
    }
    let mut ticker = tokio::time::interval(Duration::from_secs(args.interval_secs));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // A failed poll (shop down, no network) is not fatal: try again next tick.
                if let Err(e) = poller.run_once().await {
                    tracing::warn!("poll failed: {e:#}");
                }
            }
            _ = tokio::signal::ctrl_c() => return Ok(()),
        }
    }
}
