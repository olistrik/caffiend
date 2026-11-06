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
    /// What the shop says about the product (origin, size, ...), in the shop's order. A state
    /// file from before this was stored has none until the next poll.
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

/// A WooCommerce product attribute: a name and the terms it has, such as `Origin: Colombia`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub terms: Vec<String>,
}

impl Product {
    /// True when no categories are wanted, or the product is in one of them.
    pub fn in_any_category(&self, wanted: &[String]) -> bool {
        wanted.is_empty() || self.categories.iter().any(|c| wanted.contains(c))
    }

    /// The attributes to show: all of them when none are named, otherwise the named ones in the
    /// order given (names match regardless of case), skipping what the product lacks.
    pub fn shown_attributes(&self, wanted: &[String]) -> Vec<&Attribute> {
        let has_terms = |a: &&Attribute| !a.terms.is_empty();
        if wanted.is_empty() {
            return self.attributes.iter().filter(has_terms).collect();
        }
        wanted
            .iter()
            .filter_map(|name| {
                self.attributes
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(name))
            })
            .filter(has_terms)
            .collect()
    }
}
