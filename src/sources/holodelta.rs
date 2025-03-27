use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::DeckType;

use super::json::{JsonExport, JsonImport};
use super::{
    CardsInfo, CommonCard, CommonCardConversion, CommonDeck, CommonDeckConversion, MergeCommonCards,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCard(String, u32, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    #[serde(skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,
    oshi: OshiCard,
    deck: Vec<DeckCard>,
    cheer_deck: Vec<DeckCard>,
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

    fn from_common_card(card: CommonCard, info: &CardsInfo) -> Self {
        OshiCard(card.card_number.clone(), card.delta_art_index(info))
    }

    fn to_common_card(value: Self, info: &CardsInfo) -> CommonCard {
        CommonCard::from_card_number_and_delta_art_index(value.0, value.1, 1, info)
    }

    fn build_custom_deck(_cards: Vec<CommonCard>, _info: &CardsInfo) -> Self::CardDeck {
        unimplemented!("not needed for single card")
    }

    fn build_common_deck(_cards: Self::CardDeck, _info: &CardsInfo) -> Vec<CommonCard> {
        unimplemented!("not needed for single card")
    }
}

impl CommonCardConversion for DeckCard {
    type CardDeck = Vec<DeckCard>;

    fn from_common_card(card: CommonCard, info: &CardsInfo) -> Self {
        DeckCard(
            card.card_number.clone(),
            card.amount,
            card.delta_art_index(info),
        )
    }

    fn to_common_card(value: Self, info: &CardsInfo) -> CommonCard {
        CommonCard::from_card_number_and_delta_art_index(value.0, value.2, value.1, info)
    }

    fn build_custom_deck(cards: Vec<CommonCard>, info: &CardsInfo) -> Self::CardDeck {
        cards
            .merge()
            .into_iter()
            .map(|c| DeckCard::from_common_card(c, info))
            .collect()
    }

    fn build_common_deck(cards: Self::CardDeck, info: &CardsInfo) -> Vec<CommonCard> {
        cards
            .into_iter()
            .map(|c| DeckCard::to_common_card(c, info))
            .collect::<Vec<_>>()
            .merge()
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, info: &CardsInfo) -> Option<Self> {
        Some(Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_card(deck.oshi?, info),
            deck: DeckCard::build_custom_deck(deck.main_deck, info),
            cheer_deck: DeckCard::build_custom_deck(deck.cheer_deck, info),
        })
    }

    fn to_common_deck(value: Self, info: &CardsInfo) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: Some(OshiCard::to_common_card(value.oshi, info)),
            main_deck: DeckCard::build_common_deck(value.deck, info),
            cheer_deck: DeckCard::build_common_deck(value.cheer_deck, info),
        }
    }
}

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    info: Signal<CardsInfo>,
    show_price: Signal<bool>,
) -> Element {
    rsx! {
        JsonImport {
            deck_type: DeckType::HoloDelta,
            fallback_deck_type: DeckType::HoloDelta,
            import_name: "holoDelta",
            common_deck,
            info,
            show_price,
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<CommonDeck>, info: Signal<CardsInfo>) -> Element {
    rsx! {
        JsonExport {
            deck_type: DeckType::HoloDelta,
            export_name: "holoDelta",
            export_id: "holodelta",
            common_deck,
            info,
        }
    }
}
