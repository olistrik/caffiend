//! The configuration file: which shops to watch and where to send messages.
//!
//! Secrets are not in it, only the paths of files holding them.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Seconds between polls of every shop.
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    /// Holds one `<shop>.json` of remembered products per shop.
    pub state_dir: PathBuf,
    pub ntfy: Option<NtfyConfig>,
    pub telegram: Option<TelegramConfig>,
    /// The shops to watch, by a short key used for the state file and in logs.
    pub shops: BTreeMap<String, ShopConfig>,
}

fn default_interval() -> u64 {
    600
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NtfyConfig {
    #[serde(default = "default_ntfy_server")]
    pub server: String,
    /// File holding the topic. On a public server the topic is the only secret.
    pub topic_file: PathBuf,
}

fn default_ntfy_server() -> String {
    "https://ntfy.sh".to_owned()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramConfig {
    /// File holding the bot token from @BotFather.
    pub token_file: PathBuf,
    pub chat_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShopConfig {
    /// The shop's address; its WooCommerce Store API is read from there.
    pub url: String,
    /// What messages call the shop. Defaults to its hostname.
    pub name: Option<String>,
    /// Only products in these category slugs. Empty means every product.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Which attributes to show in messages, in this order. Empty means all of them.
    #[serde(default)]
    pub attributes: Vec<String>,
    /// Also announce sold-out products coming back in stock.
    #[serde(default)]
    pub restocks: bool,
}

impl ShopConfig {
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        reqwest::Url::parse(&self.url)
            .ok()
            .and_then(|url| {
                url.host_str()
                    .map(|h| h.trim_start_matches("www.").to_owned())
            })
            .unwrap_or_else(|| self.url.clone())
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("in {}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let config: Config = serde_json::from_str(text).context("invalid configuration")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        ensure!(self.interval_secs > 0, "interval_secs must be above zero");
        ensure!(!self.shops.is_empty(), "no shops to watch");
        for (key, shop) in &self.shops {
            // The key becomes a file name.
            ensure!(
                !key.is_empty()
                    && key.chars().all(|c| c.is_ascii_lowercase()
                        || c.is_ascii_digit()
                        || c == '-'
                        || c == '_'),
                "shop key {key:?} may only hold a-z, 0-9, - and _"
            );
            match reqwest::Url::parse(&shop.url) {
                Ok(url) if matches!(url.scheme(), "http" | "https") => {}
                _ => bail!("shop {key}: {:?} is not an http(s) address", shop.url),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
        "state_dir": "/var/lib/caffiend",
        "shops": { "dak": { "url": "https://www.dakcoffeeroasters.com" } }
    }"#;

    #[test]
    fn a_minimal_config_gets_defaults() {
        let config = Config::parse(MINIMAL).unwrap();
        assert_eq!(config.interval_secs, 600);
        assert!(config.ntfy.is_none() && config.telegram.is_none());
        let shop = &config.shops["dak"];
        assert!(shop.categories.is_empty() && shop.attributes.is_empty() && !shop.restocks);
        assert_eq!(shop.display_name(), "dakcoffeeroasters.com");
    }

    #[test]
    fn a_named_shop_uses_its_name() {
        let text = MINIMAL.replace(r#"{ "url""#, r#"{ "name": "DAK", "url""#);
        assert_eq!(
            Config::parse(&text).unwrap().shops["dak"].display_name(),
            "DAK"
        );
    }

    #[test]
    fn mistakes_are_rejected() {
        let typo = MINIMAL.replace("state_dir", "state_dri");
        assert!(Config::parse(&typo).is_err());
        let key = MINIMAL.replace(r#""dak""#, r#""../dak""#);
        assert!(Config::parse(&key).is_err());
        let url = MINIMAL.replace("https://www.dakcoffeeroasters.com", "dakcoffeeroasters.com");
        assert!(Config::parse(&url).is_err());
        let none = r#"{ "state_dir": "/x", "shops": {} }"#;
        assert!(Config::parse(none).is_err());
    }
}
