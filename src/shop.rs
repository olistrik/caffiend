//! The shop's public WooCommerce Store API, which lists products with their stock status.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    entities::decode,
    product::{Attribute, Product},
};

const PAGE_SIZE: usize = 100;

#[derive(Deserialize)]
struct ApiProduct {
    id: u64,
    name: String,
    permalink: String,
    is_in_stock: bool,
    categories: Vec<ApiCategory>,
    prices: ApiPrices,
    #[serde(default)]
    attributes: Vec<ApiAttribute>,
}

#[derive(Deserialize)]
struct ApiAttribute {
    name: String,
    terms: Vec<ApiTerm>,
}

#[derive(Deserialize)]
struct ApiTerm {
    name: String,
}

#[derive(Deserialize)]
struct ApiCategory {
    slug: String,
}

#[derive(Deserialize)]
struct ApiPrices {
    price: String,
    currency_code: String,
    currency_minor_unit: u32,
}

impl From<ApiAttribute> for Attribute {
    fn from(a: ApiAttribute) -> Self {
        Attribute {
            name: decode(&a.name),
            terms: a.terms.into_iter().map(|t| decode(&t.name)).collect(),
        }
    }
}

impl From<ApiProduct> for Product {
    fn from(p: ApiProduct) -> Self {
        Product {
            id: p.id,
            name: decode(&p.name),
            url: p.permalink,
            in_stock: p.is_in_stock,
            price: format_price(&p.prices),
            categories: p.categories.into_iter().map(|c| c.slug).collect(),
            attributes: p
                .attributes
                .into_iter()
                .map(Attribute::from)
                .filter(|a| !a.terms.is_empty())
                .collect(),
        }
    }
}

/// `"4490"` with two minor units and `EUR` becomes `44.90 EUR`.
fn format_price(prices: &ApiPrices) -> Option<String> {
    let minor: u64 = prices.price.parse().ok()?;
    let scale = 10_u64.checked_pow(prices.currency_minor_unit)?;
    let width = prices.currency_minor_unit as usize;
    Some(format!(
        "{}.{:0width$} {}",
        minor / scale,
        minor % scale,
        prices.currency_code
    ))
}

/// Parses one page of the Store API's product list.
pub fn parse_page(body: &str) -> Result<Vec<Product>> {
    let page: Vec<ApiProduct> = serde_json::from_str(body).context("unexpected product list")?;
    Ok(page.into_iter().map(Product::from).collect())
}

pub struct Shop {
    client: reqwest::Client,
    base_url: String,
}

impl Shop {
    pub fn new(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("caffiend/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("building the HTTP client")?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// Every product in the shop, following pagination.
    pub async fn products(&self) -> Result<Vec<Product>> {
        let mut all = Vec::new();
        for page in 1.. {
            let url = format!(
                "{}/wp-json/wc/store/v1/products?per_page={PAGE_SIZE}&page={page}",
                self.base_url
            );
            let body = self
                .client
                .get(&url)
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .with_context(|| format!("fetching {url}"))?
                .text()
                .await
                .with_context(|| format!("reading {url}"))?;
            let batch = parse_page(&body)?;
            let last = batch.len() < PAGE_SIZE;
            all.extend(batch);
            if last {
                break;
            }
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_store_api_fixture() {
        let products = parse_page(include_str!("../tests/fixtures/products.json")).unwrap();
        assert_eq!(products.len(), 3);
        let lady = &products[0];
        assert_eq!(lady.name, "Lady in Red");
        assert!(!lady.in_stock);
        assert_eq!(lady.price.as_deref(), Some("44.90 EUR"));
        assert_eq!(lady.categories, ["filter"]);
        let attribute = |name: &str| {
            lady.attributes
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.terms.clone())
        };
        assert_eq!(attribute("Origin"), Some(vec!["Colombia".to_owned()]));
        assert_eq!(
            attribute("Tasting notes"),
            Some(vec![
                "Black Cherry".into(),
                "Citrus".into(),
                "Strawberry Candy".into()
            ])
        );
        assert_eq!(lady.attributes.len(), 7);
        // A product without attributes still parses.
        assert!(products[2].attributes.is_empty());
    }

    #[test]
    fn rejects_an_error_document() {
        assert!(parse_page(r#"{"code":"rest_no_route"}"#).is_err());
    }
}
