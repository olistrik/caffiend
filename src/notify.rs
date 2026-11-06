//! Where messages go. A new way to reach the phone (Signal, say) is a new variant here.

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::{diff::Event, product::Product};

pub struct Message {
    pub title: String,
    pub body: String,
    pub url: String,
}

impl From<&Event> for Message {
    fn from(event: &Event) -> Self {
        let (verb, product) = match event {
            Event::New(p) => ("New coffee", p),
            Event::Restocked(p) => ("Back in stock", p),
        };
        Message {
            title: format!("{verb}: {}", product.name),
            body: body(product),
            url: product.url.clone(),
        }
    }
}

/// One line per fact, left out when the shop does not say: where it is from and how it was
/// processed, what it tastes like, its flavour tags, then price and sizes, then the link.
fn body(product: &Product) -> String {
    let d = &product.details;
    let text = |o: &Option<String>| o.clone().unwrap_or_default();
    let origin = joined(
        " | ",
        &[
            joined(", ", &[text(&d.origin), text(&d.region)]),
            joined(" ", &[text(&d.varietal), text(&d.process)]),
        ],
    );
    let sold_out = if product.in_stock {
        ""
    } else {
        "sold out already"
    };
    let price = joined(
        " | ",
        &[
            text(&product.price),
            d.sizes.join(" / "),
            sold_out.to_owned(),
        ],
    );
    joined(
        "\n",
        &[
            origin,
            d.tasting_notes.join(", "),
            d.flavours.join(", "),
            price,
            product.url.clone(),
        ],
    )
}

/// Joins the parts that are not empty.
fn joined(separator: &str, parts: &[String]) -> String {
    let present: Vec<&str> = parts
        .iter()
        .map(String::as_str)
        .filter(|p| !p.is_empty())
        .collect();
    present.join(separator)
}

pub enum Notifier {
    /// Only logs. Used when nothing is configured, so the first run is harmless.
    Log,
    Ntfy {
        client: reqwest::Client,
        server: String,
        topic: String,
    },
    Telegram {
        client: reqwest::Client,
        token: String,
        chat_id: String,
    },
    /// Every one of these. One failing does not stop the others; the first error is returned.
    Many(Vec<Notifier>),
}

/// A secret kept in a file, so it never lands on a command line or in the Nix store.
pub fn read_secret(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let secret = text.trim().to_owned();
    if secret.is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(secret)
}

impl Notifier {
    pub async fn send(&self, message: &Message) -> Result<()> {
        Box::pin(self.send_inner(message)).await
    }

    async fn send_inner(&self, message: &Message) -> Result<()> {
        match self {
            Notifier::Many(all) => {
                let mut first_error = None;
                for notifier in all {
                    if let Err(e) = notifier.send(message).await {
                        tracing::warn!("{e:#}");
                        first_error.get_or_insert(e);
                    }
                }
                first_error.map_or(Ok(()), Err)
            }
            Notifier::Log => {
                tracing::info!(title = %message.title, body = %message.body, "would notify");
                Ok(())
            }
            Notifier::Ntfy {
                client,
                server,
                topic,
            } => {
                let payload = json!({
                    "topic": topic,
                    "title": message.title,
                    "message": message.body,
                    "click": message.url,
                    "tags": ["coffee"],
                });
                post(client, server, &payload).await
            }
            Notifier::Telegram {
                client,
                token,
                chat_id,
            } => {
                let payload = json!({
                    "chat_id": chat_id,
                    "text": format!("{}\n{}", message.title, message.body),
                });
                post(
                    client,
                    &format!("https://api.telegram.org/bot{token}/sendMessage"),
                    &payload,
                )
                .await
            }
        }
    }
}

async fn post(client: &reqwest::Client, url: &str, payload: &serde_json::Value) -> Result<()> {
    client
        .post(url)
        .json(payload)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        // reqwest's error text includes the URL, which holds the Telegram token: drop it.
        .map_err(|e| anyhow::anyhow!("notification failed: {}", e.without_url()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::product::Details;

    fn lady_in_red(in_stock: bool) -> Product {
        Product {
            id: 1,
            name: "Lady in Red".into(),
            url: "https://example.com/lady".into(),
            in_stock,
            price: Some("44.90 EUR".into()),
            categories: vec!["filter".into()],
            details: Details {
                origin: Some("Colombia".into()),
                region: Some("Bruselas".into()),
                varietal: Some("Pacamara".into()),
                process: Some("Advanced Anaerobic Natural".into()),
                tasting_notes: vec!["Black Cherry".into(), "Citrus".into()],
                flavours: vec!["Funky".into(), "Unusual".into()],
                sizes: vec!["125g".into()],
            },
        }
    }

    #[test]
    fn a_new_coffee_message_has_the_details() {
        let message = Message::from(&Event::New(lady_in_red(false)));
        assert_eq!(message.title, "New coffee: Lady in Red");
        assert_eq!(
            message.body,
            "Colombia, Bruselas | Pacamara Advanced Anaerobic Natural\n\
             Black Cherry, Citrus\n\
             Funky, Unusual\n\
             44.90 EUR | 125g | sold out already\n\
             https://example.com/lady"
        );
    }

    #[test]
    fn a_product_without_details_still_has_price_and_link() {
        let mut product = lady_in_red(true);
        product.details = Details::default();
        let message = Message::from(&Event::Restocked(product));
        assert_eq!(message.title, "Back in stock: Lady in Red");
        assert_eq!(message.body, "44.90 EUR\nhttps://example.com/lady");
    }
}
