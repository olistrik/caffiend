//! One poll: fetch, compare, notify, remember.

use std::path::Path;

use anyhow::Result;

use crate::{
    diff::diff,
    notify::{Message, Notifier},
    shop::Shop,
    state::State,
};

pub struct Poller<'a> {
    pub shop: &'a Shop,
    pub notifier: &'a Notifier,
    pub state_path: &'a Path,
    pub categories: &'a [String],
    /// Also announce coffees that were sold out and are back.
    pub restocks: bool,
}

impl Poller<'_> {
    pub async fn run_once(&self) -> Result<()> {
        let fetched: Vec<_> = self
            .shop
            .products()
            .await?
            .into_iter()
            .filter(|p| p.in_any_category(self.categories))
            .collect();

        let Some(mut state) = State::load(self.state_path)? else {
            tracing::info!(
                products = fetched.len(),
                "first run: remembering the shop, no messages"
            );
            let state = State {
                products: fetched.into_iter().map(|p| (p.id, p)).collect(),
            };
            return state.save(self.state_path);
        };

        let events = diff(&state.products, &fetched, self.restocks);
        let mut failed = Vec::new();
        for event in &events {
            if let Err(e) = self.notifier.send(&Message::from(event)).await {
                tracing::warn!(product = %event.product().name, "{e:#}");
                failed.push(event.product().id);
            }
        }

        // A product whose message failed stays as it was, so it is announced again next time.
        for product in fetched {
            if !failed.contains(&product.id) {
                state.products.insert(product.id, product);
            }
        }
        tracing::info!(events = events.len(), failed = failed.len(), "polled");
        state.save(self.state_path)
    }
}
