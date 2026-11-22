//! Product catalog helpers, shared by the CSV, the card search filter and the card details.
//!
//! Everything comes from `hocg_products.json` (the [`ProductsDatabase`]) and the printings'
//! expansion codes, so new products need no change here. The catalog is optional: every helper
//! works with an empty one.

use hocg_fan_sim_assets_model::{CardIllustration, Product, ProductsDatabase};
use itertools::Itertools;

use crate::CardLanguage;

pub fn other_lang(language: CardLanguage) -> CardLanguage {
    match language {
        CardLanguage::Japanese => CardLanguage::English,
        CardLanguage::English => CardLanguage::Japanese,
    }
}

/// Expansion codes of the products a printing is released in, in a language, in page order.
pub fn product_codes(illust: &CardIllustration, language: CardLanguage) -> &[String] {
    illust
        .products
        .value(language.into())
        .as_deref()
        .unwrap_or_default()
}

/// The language whose products describe the printing: `language`, or the other one for a
/// printing not released in it (expansion codes are the same in both languages).
pub fn products_language(illust: &CardIllustration, language: CardLanguage) -> CardLanguage {
    if product_codes(illust, language).is_empty() {
        other_lang(language)
    } else {
        language
    }
}

/// The official name in a language, else in the other language.
pub fn official_name(product: &Product, language: CardLanguage) -> Option<&str> {
    product
        .name
        .value(language.into())
        .as_deref()
        .or(product.name.value(other_lang(language).into()).as_deref())
}

/// The release date in a language, else in the other language.
pub fn release_date(product: &Product, language: CardLanguage) -> Option<&str> {
    product
        .release_date
        .value(language.into())
        .as_deref()
        .or(product
            .release_date
            .value(other_lang(language).into())
            .as_deref())
}

/// The product page in a language, else in the other language.
pub fn product_url(product: &Product, language: CardLanguage) -> Option<&str> {
    product
        .url
        .value(language.into())
        .as_deref()
        .or(product.url.value(other_lang(language).into()).as_deref())
}

/// How to show a product in a language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductLabel {
    /// Short name, e.g. "Blooming Radiance", or the English translation of a product released
    /// only in Japanese.
    pub label: String,
    /// Full official name, for a tooltip.
    pub title: String,
    /// Show the expansion code with the name: for boosters and decks, where the code is how
    /// players refer to the product ("hBP01", "hSD01").
    pub show_code: bool,
}

/// The short name in a language (or the translation of a product released only in the other
/// language), else the other language's.
pub fn product_label(code: &str, product: &Product, language: CardLanguage) -> ProductLabel {
    let (label, _official) = product
        .display_name(language.into(), true)
        .or(product.display_name(other_lang(language).into(), true))
        .unwrap_or((code, true));
    ProductLabel {
        label: label.to_string(),
        title: official_name(product, language).unwrap_or(code).to_string(),
        show_code: CODE_CATEGORIES.contains(&product.category.as_deref().unwrap_or_default()),
    }
}

/// The official site's categories (its tokens), in the order products are listed: boosters,
/// decks, then accessories (sets and collections) and PR cards. A category the site adds later
/// is listed after these.
const CATEGORY_ORDER: [&str; 4] = ["boosters", "decks", "accessories", "pr"];

/// Categories whose products are referred to by their expansion code.
const CODE_CATEGORIES: [&str; 2] = ["boosters", "decks"];

/// The catalog in release order, by each product's first release in any language, so both
/// languages list the products in the same order. Products without a date come last.
pub fn products_by_release(catalog: &ProductsDatabase) -> Vec<CatalogEntry<'_>> {
    catalog
        .iter()
        .sorted_by_key(|(code, product)| {
            let date = [CardLanguage::Japanese, CardLanguage::English]
                .into_iter()
                .filter_map(|l| product.release_date.value(l.into()).as_deref())
                .min();
            (date.is_none(), date, *code)
        })
        .collect()
}

/// A label for a category token of the official site ("boosters", "pr", ...), which may change:
/// short tokens are abbreviations ("PR"), others are capitalized ("Boosters").
pub fn category_label(token: &str) -> String {
    if token.chars().count() <= 2 {
        token.to_uppercase()
    } else {
        let mut chars = token.chars();
        chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect())
            .unwrap_or_default()
    }
}

/// A product of the catalog, with its expansion code.
pub type CatalogEntry<'a> = (&'a String, &'a Product);

/// The products of a category (`None` without a category).
pub type CategoryGroup<'a> = (Option<&'a str>, Vec<CatalogEntry<'a>>);

/// The catalog grouped by category (see [`CATEGORY_ORDER`]), in release order. Unknown
/// categories come after the known ones, and products without a category last, under `None`.
pub fn products_by_category(catalog: &ProductsDatabase) -> Vec<CategoryGroup<'_>> {
    let mut groups: Vec<CategoryGroup> = vec![];
    for (code, product) in products_by_release(catalog) {
        let category = product.category.as_deref();
        match groups.iter_mut().find(|(c, _)| *c == category) {
            Some((_, products)) => products.push((code, product)),
            None => groups.push((category, vec![(code, product)])),
        }
    }
    groups.sort_by_key(|(category, _)| match category {
        Some(category) => match CATEGORY_ORDER.iter().position(|c| c == category) {
            Some(position) => (0, position),
            None => (1, 0),
        },
        None => (2, 0),
    });
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_order_is_the_same_in_both_languages() {
        use hocg_fan_sim_assets_model::Localized;
        let product = |jp: Option<&str>, en: Option<&str>| {
            let mut release_date = Localized::default();
            if let Some(jp) = jp {
                *release_date.value_mut(CardLanguage::Japanese.into()) = Some(jp.into());
            }
            if let Some(en) = en {
                *release_date.value_mut(CardLanguage::English.into()) = Some(en.into());
            }
            Product {
                release_date,
                ..Default::default()
            }
        };
        let catalog: ProductsDatabase = [
            ("hSD01", product(Some("2024-09-20"), Some("2025-07-11"))),
            ("hSD02", product(Some("2024-12-20"), Some("2025-10-17"))),
            ("hSD2025summer", product(Some("2025-08-16"), None)),
            ("hSD08", product(Some("2025-08-29"), None)),
            ("later", product(None, Some("2026-01-01"))),
            ("undated", product(None, None)),
        ]
        .into_iter()
        .map(|(code, product)| (code.to_string(), product))
        .collect();
        let order = products_by_release(&catalog)
            .into_iter()
            .map(|(code, _)| code.as_str())
            .collect_vec();
        assert_eq!(
            order,
            vec![
                "hSD01",
                "hSD02",
                "hSD2025summer",
                "hSD08",
                "later",
                "undated"
            ]
        );
    }

    #[test]
    fn category_labels() {
        assert_eq!(category_label("pr"), "PR");
        assert_eq!(category_label("boosters"), "Boosters");
        assert_eq!(category_label("new_thing"), "New_thing");
        assert_eq!(category_label(""), "");
    }

    #[test]
    fn category_order() {
        let product = |category: Option<&str>, date: &str| Product {
            category: category.map(String::from),
            release_date: hocg_fan_sim_assets_model::Localized::jp(date.into()),
            ..Default::default()
        };
        let catalog: ProductsDatabase = [
            ("p", product(Some("pr"), "2020-01-01")),
            ("n", product(Some("new"), "2020-01-01")),
            ("x", product(None, "2020-01-01")),
            ("a", product(Some("accessories"), "2020-01-01")),
            ("d2", product(Some("decks"), "2024-02-01")),
            ("d1", product(Some("decks"), "2024-01-01")),
            ("b", product(Some("boosters"), "2025-01-01")),
        ]
        .into_iter()
        .map(|(code, product)| (code.to_string(), product))
        .collect();
        let groups = products_by_category(&catalog)
            .into_iter()
            .map(|(category, products)| {
                (
                    category,
                    products.into_iter().map(|(c, _)| c.as_str()).collect_vec(),
                )
            })
            .collect_vec();
        assert_eq!(
            groups,
            vec![
                (Some("boosters"), vec!["b"]),
                (Some("decks"), vec!["d1", "d2"]),
                (Some("accessories"), vec!["a"]),
                (Some("pr"), vec!["p"]),
                (Some("new"), vec!["n"]),
                (None, vec!["x"]),
            ]
        );
    }
}
