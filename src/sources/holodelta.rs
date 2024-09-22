use std::error::Error;

use serde::{Deserialize, Serialize};

use super::{CommonCardEntry, CommonDeck};

type OshiCard = (String, u32);
type DeckCard = (String, u32, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    deck_name: String,
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
}

impl From<CommonCardEntry> for OshiCard {
    fn from(value: CommonCardEntry) -> Self {
        (value.card_number, value.rarity)
    }
}
impl From<OshiCard> for CommonCardEntry {
    fn from(value: OshiCard) -> Self {
        CommonCardEntry {
            card_number: value.0,
            rarity: value.1,
            amount: 1,
        }
    }
}

impl From<CommonCardEntry> for DeckCard {
    fn from(value: CommonCardEntry) -> Self {
        (value.card_number, value.amount, value.rarity)
    }
}
impl From<DeckCard> for CommonCardEntry {
    fn from(value: DeckCard) -> Self {
        CommonCardEntry {
            card_number: value.0,
            rarity: value.2,
            amount: value.1,
        }
    }
}

impl From<CommonDeck> for Deck {
    fn from(value: CommonDeck) -> Self {
        Deck {
            deck_name: value.deck_name,
            oshi: value.oshi.into(),
            deck: value.main_deck.into_iter().map(Into::into).collect(),
            cheer_deck: value.cheer_deck.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<Deck> for CommonDeck {
    fn from(value: Deck) -> Self {
        CommonDeck {
            deck_name: value.deck_name,
            oshi: value.oshi.into(),
            main_deck: value.deck.into_iter().map(Into::into).collect(),
            cheer_deck: value.cheer_deck.into_iter().map(Into::into).collect(),
        }
    }
}
