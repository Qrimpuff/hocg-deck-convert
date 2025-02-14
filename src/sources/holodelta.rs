use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::DeckType;

use super::json::{JsonExport, JsonImport};
use super::{
    CardsInfo, CommonCards, CommonCardsConversion, CommonDeck, CommonDeckConversion,
    MergeCommonCards,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCards(String, u32, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    #[serde(skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,
    oshi: OshiCard,
    deck: Vec<DeckCards>,
    cheer_deck: Vec<DeckCards>,
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
    type CardDeck = OshiCard;

    fn from_common_cards(cards: CommonCards, info: &CardsInfo) -> Self {
        OshiCard(cards.card_number.clone(), cards.rarity_order(info))
    }

    fn to_common_cards(value: Self, info: &CardsInfo) -> CommonCards {
        CommonCards::from_card_number_and_order(value.0, value.1, 1, info)
    }

    fn build_custom_deck(_cards: Vec<CommonCards>, _info: &CardsInfo) -> Self::CardDeck {
        unimplemented!("not needed for single card")
    }

    fn build_common_deck(_cards: Self::CardDeck, _info: &CardsInfo) -> Vec<CommonCards> {
        unimplemented!("not needed for single card")
    }
}

impl CommonCardsConversion for DeckCards {
    type CardDeck = Vec<DeckCards>;

    fn from_common_cards(cards: CommonCards, info: &CardsInfo) -> Self {
        DeckCards(
            cards.card_number.clone(),
            cards.amount,
            cards.rarity_order(info),
        )
    }

    fn to_common_cards(value: Self, info: &CardsInfo) -> CommonCards {
        CommonCards::from_card_number_and_order(value.0, value.2, value.1, info)
    }

    fn build_custom_deck(cards: Vec<CommonCards>, info: &CardsInfo) -> Self::CardDeck {
        cards
            .merge()
            .into_iter()
            .map(|c| DeckCards::from_common_cards(c, info))
            .collect()
    }

    fn build_common_deck(cards: Self::CardDeck, info: &CardsInfo) -> Vec<CommonCards> {
        cards
            .into_iter()
            .map(|c| DeckCards::to_common_cards(c, info))
            .collect::<Vec<_>>()
            .merge()
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, info: &CardsInfo) -> Self {
        Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_cards(deck.oshi, info),
            deck: DeckCards::build_custom_deck(deck.main_deck, info),
            cheer_deck: DeckCards::build_custom_deck(deck.cheer_deck, info),
        }
    }

    fn to_common_deck(value: Self, info: &CardsInfo) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: OshiCard::to_common_cards(value.oshi, info),
            main_deck: DeckCards::build_common_deck(value.deck, info),
            cheer_deck: DeckCards::build_common_deck(value.cheer_deck, info),
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
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, info: Signal<CardsInfo>) -> Element {
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
