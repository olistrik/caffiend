//! What changed in the shop between two polls.

use std::collections::BTreeMap;

use crate::product::Product;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A product this program has not seen before.
    New(Product),
    /// A known product that was sold out and can be bought again.
    Restocked(Product),
}

impl Event {
    pub fn product(&self) -> &Product {
        match self {
            Event::New(p) | Event::Restocked(p) => p,
        }
    }
}

/// Compares the stored products with the fetched ones, in the fetched order. Restocks are only
/// reported when asked for.
pub fn diff(stored: &BTreeMap<u64, Product>, fetched: &[Product], restocks: bool) -> Vec<Event> {
    fetched
        .iter()
        .filter_map(|product| match stored.get(&product.id) {
            None => Some(Event::New(product.clone())),
            Some(old) if restocks && !old.in_stock && product.in_stock => {
                Some(Event::Restocked(product.clone()))
            }
            Some(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(id: u64, in_stock: bool) -> Product {
        Product {
            id,
            name: format!("Coffee {id}"),
            url: format!("https://example.com/{id}"),
            in_stock,
            price: None,
            categories: vec!["filter".into()],
            details: Default::default(),
        }
    }

    fn stored(products: &[Product]) -> BTreeMap<u64, Product> {
        products.iter().map(|p| (p.id, p.clone())).collect()
    }

    #[test]
    fn reports_new_and_restocked_products_only() {
        let before = stored(&[product(1, true), product(2, false), product(3, true)]);
        let after = [
            product(1, true),
            product(2, true),
            product(3, false),
            product(4, true),
        ];
        assert_eq!(
            diff(&before, &after, true),
            [
                Event::Restocked(product(2, true)),
                Event::New(product(4, true))
            ]
        );
    }

    #[test]
    fn selling_out_is_not_an_event() {
        let before = stored(&[product(1, true)]);
        assert!(diff(&before, &[product(1, false)], true).is_empty());
    }

    #[test]
    fn restocks_are_left_out_unless_asked_for() {
        let before = stored(&[product(1, false)]);
        let after = [product(1, true), product(2, true)];
        assert_eq!(diff(&before, &after, false), [Event::New(product(2, true))]);
    }
}
