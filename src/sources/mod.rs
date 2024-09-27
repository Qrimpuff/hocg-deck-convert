use std::collections::BTreeMap;

use serde::Deserialize;

pub mod deck_log;
pub mod holodelta;
pub mod holoduel;
pub mod tabletop_sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckType {
    DeckLog,
    HoloDelta,
    HoloDuel,
    TabletopSim,
    Unknown,
}

pub trait CommonCardsConversion {
    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self;
    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards;
}

pub trait CommonDeckConversion {
    fn from_common_deck(deck: CommonDeck, map: &CardsInfoMap) -> Self;
    fn to_common_deck(value: Self, map: &CardsInfoMap) -> CommonDeck;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonCards {
    pub manage_id: Option<String>,
    pub card_number: String,
    pub amount: u32,
}

impl CommonCards {
    pub fn from_card_number(card_number: String, amount: u32, map: &CardsInfoMap) -> Self {
        let card = map
            .values()
            .find(|c| c.card_number.eq_ignore_ascii_case(&card_number));
        CommonCards {
            manage_id: card.map(|c| c.manage_id.clone()),
            card_number: card.map(|c| c.card_number.clone()).unwrap_or(card_number),
            amount,
        }
    }

    pub fn from_card_number_and_art_order(
        card_number: String,
        art_order: u32,
        amount: u32,
        map: &CardsInfoMap,
    ) -> Self {
        let card = map
            .values()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .nth(art_order as usize);
        CommonCards {
            manage_id: card.map(|c| c.manage_id.clone()),
            card_number: card.map(|c| c.card_number.clone()).unwrap_or(card_number),
            amount,
        }
    }

    pub fn art_order(&self, map: &CardsInfoMap) -> u32 {
        map.values()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&self.card_number))
            .enumerate()
            .filter(|(_, c)| Some(&c.manage_id) == self.manage_id.as_ref())
            .map(|(i, _)| i)
            .next()
            .unwrap_or_default() as u32
    }
}

#[derive(Debug, Clone)]
pub struct CommonDeck {
    pub name: Option<String>,
    pub oshi: CommonCards,
    pub main_deck: Vec<CommonCards>,
    pub cheer_deck: Vec<CommonCards>,
}

impl CommonDeck {
    pub fn required_deck_name(&self) -> String {
        if let Some(name) = &self.name {
            if name.trim().is_empty() {
                self.default_deck_name()
            } else {
                name.clone()
            }
        } else {
            self.default_deck_name()
        }
    }

    pub fn default_deck_name(&self) -> String {
        format!("Custom deck - {}", self.oshi.card_number)
    }

    pub fn file_name(&self) -> String {
        let mut name = self.required_deck_name();
        if !name.is_ascii() {
            name = self.default_deck_name();
        }

        name.trim()
            .to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' => c,
                _ => '_',
            })
            .fold(String::new(), |mut acc, ch| {
                if ch != '_' || !acc.ends_with('_') {
                    acc.push(ch);
                }
                acc
            })
    }
}

pub type CardsInfoMap = BTreeMap<u32, CardInfoEntry>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
// from https://qrimpuff.github.io/hocg-fan-sim-assets/cards_info.json
pub struct CardInfoEntry {
    pub manage_id: String,
    pub card_number: String,
    pub img: String,
    pub max: u32,
    pub deck_type: String,
}
