use std::{collections::BTreeMap, error::Error, ops::Not};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::json::{JsonExport, JsonImport};
use crate::DeckType;

use super::{CardsInfoMap, CommonCards, CommonCardsConversion, CommonDeck, CommonDeckConversion};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCards(String, u32);
impl From<(String, u32)> for DeckCards {
    fn from(value: (String, u32)) -> Self {
        DeckCards(value.0, value.1)
    }
}
impl From<DeckCards> for (String, u32) {
    fn from(value: DeckCards) -> Self {
        (value.0, value.1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Deck {
    #[serde(skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,
    oshi: OshiCard,
    deck: BTreeMap<String, u32>,
    cheer_deck: BTreeMap<String, u32>,
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

impl CommonCardsConversion for OshiCard {
    fn from_common_cards(cards: CommonCards, _map: &CardsInfoMap) -> Self {
        OshiCard(cards.card_number.clone())
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, 0, 1, map)
    }
}

impl CommonCardsConversion for DeckCards {
    fn from_common_cards(cards: CommonCards, _map: &CardsInfoMap) -> Self {
        DeckCards(cards.card_number.clone(), cards.amount)
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, 0, value.1, map)
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, map: &CardsInfoMap) -> Self {
        Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_cards(deck.oshi, map),
            deck: deck
                .main_deck
                .into_iter()
                .map(|c| DeckCards::from_common_cards(c, map).into())
                .collect(),
            cheer_deck: deck
                .cheer_deck
                .into_iter()
                .map(|c| DeckCards::from_common_cards(c, map).into())
                .collect(),
        }
    }

    fn to_common_deck(value: Self, map: &CardsInfoMap) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: OshiCard::to_common_cards(value.oshi, map),
            main_deck: value
                .deck
                .into_iter()
                .map(|c| DeckCards::to_common_cards(c.into(), map))
                .collect(),
            cheer_deck: value
                .cheer_deck
                .into_iter()
                .map(|c| DeckCards::to_common_cards(c.into(), map))
                .collect(),
        }
    }
}

#[component]
pub fn Import(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    rsx! {
        JsonImport { deck_type: DeckType::HoloDuel, import_name: "HoloDuel",  common_deck, map }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    rsx! {
        JsonExport { deck_type: DeckType::HoloDuel, export_name: "HoloDuel", export_id: "holoduel", common_deck, map }
    }
}
