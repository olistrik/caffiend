//! One poll of one shop: fetch, compare, notify, remember.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{
    config::ShopConfig,
    diff::diff,
    notify::{Message, Notifier},
    shop::Shop,
    state::State,
};

pub struct Poller {
    key: String,
    name: String,
    config: ShopConfig,
    shop: Shop,
    state_path: PathBuf,
}

impl Poller {
    pub fn new(key: &str, config: ShopConfig, state_dir: &Path) -> Result<Self> {
        Ok(Self {
            key: key.to_owned(),
            name: config.display_name(),
            shop: Shop::new(&config.url)?,
            state_path: state_dir.join(format!("{key}.json")),
            config,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub async fn run_once(&self, notifier: &Notifier) -> Result<()> {
        let fetched: Vec<_> = self
            .shop
            .products()
            .await?
            .into_iter()
            .filter(|p| p.in_any_category(&self.config.categories))
            .collect();

        let Some(mut state) = State::load(&self.state_path)? else {
            tracing::info!(shop = %self.key, products = fetched.len(), "first run: remembering the shop, no messages");
            let state = State {
                products: fetched.into_iter().map(|p| (p.id, p)).collect(),
            };
            return state.save(&self.state_path);
        };

        let events = diff(&state.products, &fetched, self.config.restocks);
        let mut failed = Vec::new();
        for event in &events {
            let message = Message::new(event, &self.name, &self.config.attributes);
            if let Err(e) = notifier.send(&message).await {
                tracing::warn!(shop = %self.key, product = %event.product().name, "{e:#}");
                failed.push(event.product().id);
            }
        }

        // A product whose message failed stays as it was, so it is announced again next time.
        for product in fetched {
            if !failed.contains(&product.id) {
                state.products.insert(product.id, product);
            }
        }
        tracing::info!(shop = %self.key, events = events.len(), failed = failed.len(), "polled");
        state.save(&self.state_path)
    }
}

/// Polls every shop. One shop failing does not stop the others; returns how many failed.
pub async fn poll_all(pollers: &[Poller], notifier: &Notifier) -> usize {
    let mut failures = 0;
    for poller in pollers {
        if let Err(e) = poller.run_once(notifier).await {
            // Shop down or no network: not fatal, the next poll tries again.
            tracing::warn!(shop = %poller.key(), "poll failed: {e:#}");
            failures += 1;
        }
    }
    failures
}
