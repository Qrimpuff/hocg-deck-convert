use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    MergeCommonCards,
    json::{JsonExport, JsonImport},
};
use crate::DeckType;

use super::{CardsInfo, CommonCard, CommonCardConversion, CommonDeck, CommonDeckConversion};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String);

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
}

impl CommonCardConversion for OshiCard {
    type CardDeck = OshiCard;

    fn from_common_card(card: CommonCard, _info: &CardsInfo) -> Self {
        OshiCard(card.card_number.clone())
    }

    fn to_common_card(value: Self, info: &CardsInfo) -> CommonCard {
        CommonCard::from_card_number(value.0, 1, info)
    }

    fn build_custom_deck(_cards: Vec<CommonCard>, _info: &CardsInfo) -> Self::CardDeck {
        unimplemented!("not needed for single card")
    }

    fn build_common_deck(_cards: Self::CardDeck, _info: &CardsInfo) -> Vec<CommonCard> {
        unimplemented!("not needed for single card")
    }
}

impl CommonCardConversion for DeckCard {
    type CardDeck = IndexMap<String, u32>;

    fn from_common_card(card: CommonCard, _info: &CardsInfo) -> Self {
        DeckCard(card.card_number.clone(), card.amount)
    }

    fn to_common_card(value: Self, info: &CardsInfo) -> CommonCard {
        CommonCard::from_card_number(value.0, value.1, info)
    }

    fn build_custom_deck(cards: Vec<CommonCard>, info: &CardsInfo) -> Self::CardDeck {
        cards
            .merge_without_rarity()
            .into_iter()
            .map(|c| DeckCard::from_common_card(c, info).into())
            .collect()
    }

    fn build_common_deck(cards: Self::CardDeck, info: &CardsInfo) -> Vec<CommonCard> {
        cards
            .into_iter()
            .map(|c| DeckCard::to_common_card(c.into(), info))
            .collect::<Vec<_>>()
            .merge()
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, info: &CardsInfo) -> Self {
        Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_card(deck.oshi, info),
            deck: DeckCard::build_custom_deck(deck.main_deck, info),
            cheer_deck: DeckCard::build_custom_deck(deck.cheer_deck, info),
        }
    }

    fn to_common_deck(value: Self, info: &CardsInfo) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: OshiCard::to_common_card(value.oshi, info),
            main_deck: DeckCard::build_common_deck(value.deck, info),
            cheer_deck: DeckCard::build_common_deck(value.cheer_deck, info),
        }
    }
}

#[component]
pub fn Import(
    mut common_deck: Signal<Option<CommonDeck>>,
    info: Signal<CardsInfo>,
    show_price: Signal<bool>,
) -> Element {
    rsx! {
        JsonImport {
            deck_type: DeckType::HoloDuel,
            fallback_deck_type: DeckType::HoloDelta,
            import_name: "HoloDuel",
            common_deck,
            info,
            show_price,
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, info: Signal<CardsInfo>) -> Element {
    rsx! {
        JsonExport {
            deck_type: DeckType::HoloDuel,
            export_name: "HoloDuel",
            export_id: "holoduel",
            common_deck,
            info,
        }
    }
}
