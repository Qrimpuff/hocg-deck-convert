use std::{error::Error, ops::Not};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::DeckType;
use crate::sources::DeckOrPile;

use super::json::{JsonExport, JsonImport};
use super::{CardsDatabase, CommonCard, CommonDeck, MergeCommonCards};

const FORMAT_PREFIX: &str = "hocg1.";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String);

impl OshiCard {
    pub fn from_common_card(card: CommonCard, db: &CardsDatabase) -> Self {
        OshiCard(card.card_number.clone())
    }

    pub fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard {
        CommonCard::from_card_number_and_delta_art_index(value.0, 0, 1, db)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCard(String, u32);

impl DeckCard {
    pub fn from_common_card(card: CommonCard, db: &CardsDatabase) -> Self {
        DeckCard(card.card_number.clone(), card.amount)
    }

    pub fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard {
        CommonCard::from_card_number_and_delta_art_index(value.0, 0, value.1, db)
    }

    pub fn build_custom_deck(cards: Vec<CommonCard>, db: &CardsDatabase) -> Vec<DeckCard> {
        cards
            .merge()
            .into_iter()
            .map(|c| DeckCard::from_common_card(c, db))
            .collect()
    }

    pub fn build_common_deck(cards: Vec<DeckCard>, db: &CardsDatabase) -> Vec<CommonCard> {
        cards
            .into_iter()
            .map(|c| DeckCard::to_common_card(c, db))
            .collect::<Vec<_>>()
            .merge()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    #[serde(rename = "n", skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,

    #[serde(rename = "o")]
    oshi: OshiCard,

    #[serde(rename = "m")]
    deck: Vec<DeckCard>,

    #[serde(rename = "c")]
    cheer_deck: Vec<DeckCard>,
}

impl Deck {
    pub fn from_file(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Self::from_text(std::str::from_utf8(bytes)?)
    }

    // Parses the raw, compact JSON.
    pub fn from_text(text: &str) -> Result<Self, Box<dyn Error>> {
        let encoded = text
            .strip_prefix(FORMAT_PREFIX)
            .ok_or("Invalid deck format prefix")?;

        let decoded = STANDARD.decode(encoded)?;
        Ok(serde_json::from_slice(&decoded)?)
    }

    pub fn to_file(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self.to_text()?.into_bytes())
    }

    // Produces compact JSON without whitespace.
    pub fn to_text(&self) -> Result<String, Box<dyn Error>> {
        let json = serde_json::to_vec(self)?;
        Ok(format!("{FORMAT_PREFIX}{}", STANDARD.encode(json)))
    }

    // Produces hocg1.BASE64(JSON).
    pub fn to_import_code(&self) -> Result<String, Box<dyn Error>> {
        let json = self.to_text()?;
        let encoded = STANDARD.encode(json.as_bytes());

        Ok(format!("{FORMAT_PREFIX}{encoded}"))
    }

    // Parses hocg1.BASE64(JSON).
    pub fn from_import_code(code: &str) -> Result<Self, Box<dyn Error>> {
        let encoded = code
            .strip_prefix(FORMAT_PREFIX)
            .ok_or("Invalid deck format prefix")?;

        let decoded = STANDARD.decode(encoded)?;
        let json = String::from_utf8(decoded)?;

        Self::from_text(&json)
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
pub fn Import(mut common_deck: Signal<DeckOrPile>, db: Signal<CardsDatabase>) -> Element {
    rsx! {
        JsonImport {
            deck_type: DeckType::HoloBuri,
            fallback_deck_type: DeckType::HoloBuri,
            import_name: "holoBuri",
            import_format: "base64",
            common_deck,
            db,
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<DeckOrPile>, db: Signal<CardsDatabase>) -> Element {
    rsx! {
        JsonExport {
            deck_type: DeckType::HoloBuri,
            export_name: "holoBuri",
            export_id: "holoburi",
            export_format: "base64",
            allow_unreleased: true,
            common_deck,
            db
        }
    }
}
