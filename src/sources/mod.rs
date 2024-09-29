use std::{
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
};

use serde::Deserialize;

pub mod deck_log;
pub mod holodelta;
pub mod holoduel;
pub mod json;
pub mod tabletop_sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckType {
    DeckLog,
    HoloDelta,
    HoloDuel,
    TabletopSim,
    Unknown,
}

pub trait CommonCardsConversion: Sized {
    type CardDeck;

    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self;
    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards;

    fn build_custom_deck(cards: Vec<CommonCards>, map: &CardsInfoMap) -> Self::CardDeck;
    fn build_common_deck(cards: Self::CardDeck, map: &CardsInfoMap) -> Vec<CommonCards>;
}

pub trait CommonDeckConversion {
    fn from_common_deck(deck: CommonDeck, map: &CardsInfoMap) -> Self;
    fn to_common_deck(value: Self, map: &CardsInfoMap) -> CommonDeck;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Hash)]
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

    pub fn validate(&self, map: &CardsInfoMap) -> Vec<String> {
        let mut errors = vec![];

        // check for unreleased or invalid cards
        if self.oshi.manage_id.is_none()
            || self.main_deck.iter().any(|c| c.manage_id.is_none())
            || self.cheer_deck.iter().any(|c| c.manage_id.is_none())
        {
            errors.push("Contains unknown cards.".into());
        }

        // check for card amount
        if self.oshi.manage_id.is_none() {
            errors.push("Missing an Oshi card.".into());
        }
        let main_deck_amount = self.main_deck.iter().map(|c| c.amount).sum::<u32>();
        if main_deck_amount > 50 {
            errors.push(format!(
                "Too many cards in main deck. ({main_deck_amount} cards)"
            ));
        }
        if main_deck_amount < 50 {
            errors.push(format!(
                "Not enough cards in main deck. ({main_deck_amount} cards)"
            ));
        }
        let cheer_deck_amount = self.cheer_deck.iter().map(|c| c.amount).sum::<u32>();
        if cheer_deck_amount > 20 {
            errors.push(format!(
                "Too many cards in cheer deck. ({cheer_deck_amount} cards)"
            ));
        }
        if cheer_deck_amount < 20 {
            errors.push(format!(
                "Not enough cards in cheer deck. ({cheer_deck_amount} cards)"
            ));
        }

        // check for unlimited cards
        // group cards by card number, to avoid miscalculation with different images
        let main_deck = self.main_deck.iter().fold(HashMap::new(), |mut acc, c| {
            *acc.entry(&c.card_number).or_default() += c.amount;
            acc
        });
        for card in main_deck
            .into_iter()
            .map(|(k, v)| CommonCards::from_card_number(k.clone(), v, map))
        {
            if card.amount
                > card
                    .manage_id
                    .and_then(|m| {
                        map.get(&m.parse().expect("should be a number"))
                            .map(|i| i.max)
                    })
                    .unwrap_or(50)
            {
                errors.push(format!(
                    "Too many {} in main deck. ({} cards)",
                    card.card_number, card.amount
                ));
            }
        }

        errors
    }

    pub fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}

// need to keep the order to know which card image to use
// (holoDelta is using a zero-based index)
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
