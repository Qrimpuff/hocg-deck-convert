use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::DeckType;

use super::json::{JsonExport, JsonImport};
use super::{CardsInfoMap, CommonCards, CommonCardsConversion, CommonDeck, CommonDeckConversion};

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

    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self {
        OshiCard(cards.card_number.clone(), cards.art_order(map))
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, value.1, 1, map)
    }

    fn build_custom_deck(_cards: Vec<CommonCards>, _map: &CardsInfoMap) -> Self::CardDeck {
        unimplemented!("not needed for single card")
    }

    fn build_common_deck(_cards: Self::CardDeck, _map: &CardsInfoMap) -> Vec<CommonCards> {
        unimplemented!("not needed for single card")
    }
}

impl CommonCardsConversion for DeckCards {
    type CardDeck = Vec<DeckCards>;

    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self {
        DeckCards(
            cards.card_number.clone(),
            cards.amount,
            cards.art_order(map),
        )
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, value.2, value.1, map)
    }

    fn build_custom_deck(cards: Vec<CommonCards>, map: &CardsInfoMap) -> Self::CardDeck {
        cards
            .into_iter()
            .map(|c| DeckCards::from_common_cards(c, map))
            .fold(IndexMap::<(String, u32), u32>::new(), |mut acc, c| {
                *acc.entry((c.0, c.2)).or_default() += c.1;
                acc
            })
            .into_iter()
            .map(|(k, v)| DeckCards(k.0, v, k.1))
            .collect()
    }

    fn build_common_deck(cards: Self::CardDeck, map: &CardsInfoMap) -> Vec<CommonCards> {
        cards
            .into_iter()
            .map(|c| DeckCards::to_common_cards(c, map))
            .collect()
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, map: &CardsInfoMap) -> Self {
        Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_cards(deck.oshi, map),
            deck: DeckCards::build_custom_deck(deck.main_deck, map),
            cheer_deck: DeckCards::build_custom_deck(deck.cheer_deck, map),
        }
    }

    fn to_common_deck(value: Self, map: &CardsInfoMap) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: OshiCard::to_common_cards(value.oshi, map),
            main_deck: DeckCards::build_common_deck(value.deck, map),
            cheer_deck: DeckCards::build_common_deck(value.cheer_deck, map),
        }
    }
}

#[component]
pub fn Import(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    rsx! {
        JsonImport {
            deck_type: DeckType::HoloDelta,
            import_name: "holoDelta",
            common_deck,
            map
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    rsx! {
        JsonExport {
            deck_type: DeckType::HoloDelta,
            export_name: "holoDelta",
            export_id: "holodelta",
            common_deck,
            map
        }
    }
}
