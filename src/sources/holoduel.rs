use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    MergeCommonCards,
    json::{JsonExport, JsonImport},
};
use crate::DeckType;

use super::{CardsDatabase, CommonCard, CommonDeck};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String);

impl OshiCard {
    pub fn from_common_card(card: CommonCard, _db: &CardsDatabase) -> Self {
        OshiCard(card.card_number.clone())
    }

    pub fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard {
        CommonCard::from_card_number(value.0, 1, db)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCard(String, u32);
impl From<(String, u32)> for DeckCard {
    fn from(value: (String, u32)) -> Self {
        DeckCard(value.0, value.1)
    }
}
impl From<DeckCard> for (String, u32) {
    fn from(value: DeckCard) -> Self {
        (value.0, value.1)
    }
}

impl DeckCard {
    pub fn from_common_card(card: CommonCard, _db: &CardsDatabase) -> Self {
        DeckCard(card.card_number.clone(), card.amount)
    }

    pub fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard {
        CommonCard::from_card_number(value.0, value.1, db)
    }

    pub fn build_custom_deck(cards: Vec<CommonCard>, db: &CardsDatabase) -> IndexMap<String, u32> {
        cards
            .merge_without_rarity()
            .into_iter()
            .map(|c| DeckCard::from_common_card(c, db).into())
            .collect()
    }

    pub fn build_common_deck(cards: IndexMap<String, u32>, db: &CardsDatabase) -> Vec<CommonCard> {
        cards
            .into_iter()
            .map(|c| DeckCard::to_common_card(c.into(), db))
            .collect::<Vec<_>>()
            .merge()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Deck {
    #[serde(skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,
    oshi: OshiCard,
    deck: IndexMap<String, u32>,
    cheer_deck: IndexMap<String, u32>,
}

impl Deck {
    pub fn from_file(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }

    pub fn from_text(text: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn to_file(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(serde_json::to_vec(&self)?)
    }

    pub fn to_text(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self)?)
    }

    pub fn from_common_deck(deck: CommonDeck, db: &CardsDatabase) -> Option<Self> {
        Some(Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_card(deck.oshi?, db),
            deck: DeckCard::build_custom_deck(deck.main_deck, db),
            cheer_deck: DeckCard::build_custom_deck(deck.cheer_deck, db),
        })
    }

    pub fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: Some(OshiCard::to_common_card(value.oshi, db)),
            main_deck: DeckCard::build_common_deck(value.deck, db),
            cheer_deck: DeckCard::build_common_deck(value.cheer_deck, db),
        }
    }
}

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
    show_price: Signal<bool>,
) -> Element {
    rsx! {
        JsonImport {
            deck_type: DeckType::HoloDuel,
            fallback_deck_type: DeckType::HoloDelta,
            import_name: "HoloDuel",
            common_deck,
            db,
            show_price,
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<CommonDeck>, db: Signal<CardsDatabase>) -> Element {
    rsx! {
        JsonExport {
            deck_type: DeckType::HoloDuel,
            export_name: "HoloDuel",
            export_id: "holoduel",
            allow_unreleased: false,
            common_deck,
            db,
        }
    }
}
