//! The shop's products as this program sees them: what is stored and compared between polls.

use serde::{Deserialize, Serialize};

/// One product at one moment. Only what notifications and a later frontend need.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub in_stock: bool,
    /// Display price, such as `44.90 EUR`. For variable products this is the lowest variant's.
    pub price: Option<String>,
    /// Category slugs, such as `filter` or `espresso`.
    pub categories: Vec<String>,
    /// What the shop says about the coffee. All optional: older state files and non-coffee
    /// products lack them.
    #[serde(default)]
    pub details: Details,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Details {
    pub origin: Option<String>,
    pub region: Option<String>,
    pub varietal: Option<String>,
    pub process: Option<String>,
    pub tasting_notes: Vec<String>,
    /// The shop's own flavour tags, such as `Funky` or `Bright`.
    pub flavours: Vec<String>,
    /// Bag sizes, such as `250g`.
    pub sizes: Vec<String>,
}

impl Product {
    /// True when no categories are wanted, or the product is in one of them.
    pub fn in_any_category(&self, wanted: &[String]) -> bool {
        wanted.is_empty() || self.categories.iter().any(|c| wanted.contains(c))
    }
}
