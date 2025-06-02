use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::json::{JsonExport, JsonImport};

use super::{
    CardsDatabase, CommonCard, CommonCardConversion, CommonDeck, CommonDeckConversion,
    MergeCommonCards,
};
use crate::DeckType;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard([String; 1]);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCard(String, u32);

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

    fn from_common_card(card: CommonCard, _db: &CardsDatabase) -> Self {
        OshiCard([card.card_number.clone()])
    }

    fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard {
        CommonCard::from_card_number(value.0[0].clone(), 1, db)
    }

    fn build_custom_deck(_cards: Vec<CommonCard>, _db: &CardsDatabase) -> Self::CardDeck {
        unimplemented!("not needed for single card")
    }

    fn build_common_deck(_cards: Self::CardDeck, _db: &CardsDatabase) -> Vec<CommonCard> {
        unimplemented!("not needed for single card")
    }
}

impl CommonCardConversion for DeckCard {
    type CardDeck = Vec<DeckCard>;

    fn from_common_card(card: CommonCard, _db: &CardsDatabase) -> Self {
        DeckCard(card.card_number.clone(), card.amount)
    }

    fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard {
        CommonCard::from_card_number(value.0, value.1, db)
    }

    fn build_custom_deck(cards: Vec<CommonCard>, db: &CardsDatabase) -> Self::CardDeck {
        cards
            .merge_without_rarity()
            .into_iter()
            .map(|c| DeckCard::from_common_card(c, db))
            .collect()
    }

    fn build_common_deck(cards: Self::CardDeck, db: &CardsDatabase) -> Vec<CommonCard> {
        cards
            .into_iter()
            .map(|c| DeckCard::to_common_card(c, db))
            .collect::<Vec<_>>()
            .merge()
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, db: &CardsDatabase) -> Option<Self> {
        Some(Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_card(deck.oshi?, db),
            deck: DeckCard::build_custom_deck(deck.main_deck, db),
            cheer_deck: DeckCard::build_custom_deck(deck.cheer_deck, db),
        })
    }

    fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck {
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
            deck_type: DeckType::TabletopSim,
            fallback_deck_type: DeckType::HoloDelta,
            import_name: "Tabletop Sim",
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
            deck_type: DeckType::TabletopSim,
            export_name: "Tabletop Sim",
            export_id: "hocg_tts",
            allow_unreleased: false,
            common_deck,
            db,
        }
    }
}
