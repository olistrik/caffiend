//! Where messages go. A new way to reach the phone (Signal, say) is a new variant here.

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::{config::Config, diff::Event, product::Product};

pub struct Message {
    pub title: String,
    pub body: String,
    /// Where tapping the message goes. Empty for none.
    pub url: String,
}

impl Message {
    /// `shop` is what the shop is called; `shown` names the attributes to include (all when empty).
    pub fn new(event: &Event, shop: &str, shown: &[String]) -> Self {
        let (verb, product) = match event {
            Event::New(p) => ("New", p),
            Event::Restocked(p) => ("Back in stock", p),
        };
        Message {
            title: format!("{verb} at {shop}: {}", product.name),
            body: body(product, shown),
            url: product.url.clone(),
        }
    }

    pub fn test() -> Self {
        Message {
            title: "caffiend test".to_owned(),
            body: "If you can read this, notifications work.".to_owned(),
            url: String::new(),
        }
    }
}

/// One line per attribute (`Origin: Colombia`), then price, then the link.
fn body(product: &Product, shown: &[String]) -> String {
    let mut lines: Vec<String> = product
        .shown_attributes(shown)
        .into_iter()
        .map(|a| format!("{}: {}", a.name, a.terms.join(", ")))
        .collect();
    let mut price: Vec<&str> = product.price.as_deref().into_iter().collect();
    if !product.in_stock {
        price.push("sold out already");
    }
    if !price.is_empty() {
        lines.push(price.join(" | "));
    }
    lines.push(product.url.clone());
    lines.join("\n")
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
fn read_secret(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let secret = text.trim().to_owned();
    if secret.is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(secret)
}

impl Notifier {
    pub fn from_config(config: &Config) -> Result<Self> {
        let mut all = Vec::new();
        if let Some(ntfy) = &config.ntfy {
            all.push(Notifier::Ntfy {
                client: reqwest::Client::new(),
                server: ntfy.server.clone(),
                topic: read_secret(&ntfy.topic_file)?,
            });
        }
        if let Some(telegram) = &config.telegram {
            all.push(Notifier::Telegram {
                client: reqwest::Client::new(),
                token: read_secret(&telegram.token_file)?,
                chat_id: telegram.chat_id.clone(),
            });
        }
        if all.is_empty() {
            tracing::warn!("no notifier configured: messages are only logged");
            all.push(Notifier::Log);
        }
        Ok(Notifier::Many(all))
    }

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
                let mut payload = json!({
                    "topic": topic,
                    "title": message.title,
                    "message": message.body,
                    "tags": ["coffee"],
                });
                if !message.url.is_empty() {
                    payload["click"] = json!(message.url);
                }
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
                let url = format!("https://api.telegram.org/bot{token}/sendMessage");
                post(client, &url, &payload).await
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
    use crate::product::Attribute;

    fn attribute(name: &str, terms: &[&str]) -> Attribute {
        Attribute {
            name: name.to_owned(),
            terms: terms.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    fn lady_in_red(in_stock: bool) -> Product {
        Product {
            id: 1,
            name: "Lady in Red".into(),
            url: "https://example.com/lady".into(),
            in_stock,
            price: Some("44.90 EUR".into()),
            categories: vec!["filter".into()],
            attributes: vec![
                attribute("Origin", &["Colombia"]),
                attribute("Altitude", &["1800masl"]),
                attribute("Tasting notes", &["Black Cherry", "Citrus"]),
                attribute("Empty", &[]),
            ],
        }
    }

    #[test]
    fn a_message_shows_the_chosen_attributes_in_order() {
        let shown = [
            "tasting notes".to_owned(),
            "Origin".to_owned(),
            "Missing".to_owned(),
        ];
        let message = Message::new(&Event::New(lady_in_red(false)), "DAK", &shown);
        assert_eq!(message.title, "New at DAK: Lady in Red");
        assert_eq!(
            message.body,
            "Tasting notes: Black Cherry, Citrus\n\
             Origin: Colombia\n\
             44.90 EUR | sold out already\n\
             https://example.com/lady"
        );
    }

    #[test]
    fn with_none_chosen_every_attribute_with_terms_is_shown() {
        let message = Message::new(&Event::Restocked(lady_in_red(true)), "DAK", &[]);
        assert_eq!(message.title, "Back in stock at DAK: Lady in Red");
        assert_eq!(
            message.body,
            "Origin: Colombia\n\
             Altitude: 1800masl\n\
             Tasting notes: Black Cherry, Citrus\n\
             44.90 EUR\n\
             https://example.com/lady"
        );
    }

    #[test]
    fn a_product_without_attributes_still_has_price_and_link() {
        let mut product = lady_in_red(true);
        product.attributes.clear();
        let message = Message::new(&Event::New(product), "DAK", &[]);
        assert_eq!(message.body, "44.90 EUR\nhttps://example.com/lady");
    }
}
