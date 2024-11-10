use std::{
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::CardLanguage;

pub mod deck_log;
pub mod holodelta;
pub mod holoduel;
pub mod json;
pub mod proxy_sheets;
pub mod tabletop_sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckType {
    DeckLog,
    HoloDelta,
    HoloDuel,
    TabletopSim,
    ProxySheets,
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

    pub fn from_card_number_and_rarity_order(
        card_number: String,
        rarity_order: u32,
        amount: u32,
        map: &CardsInfoMap,
    ) -> Self {
        // grouped by card image
        let rarities: IndexMap<_, _> = map
            .values()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .fold(Default::default(), |mut acc, c| {
                acc.entry(&c.img).or_insert(c);
                acc
            });
        let card = rarities.values().nth(rarity_order as usize);
        if let Some(card) = card {
            CommonCards {
                manage_id: Some(card.manage_id.clone()),
                card_number: card.card_number.clone(),
                amount,
            }
        } else {
            // default to basic rarity if not found
            CommonCards::from_card_number(card_number, amount, map)
        }
    }

    pub fn rarity_order(&self, map: &CardsInfoMap) -> u32 {
        if let Some(c) = map.get(
            &self
                .manage_id
                .as_ref()
                .and_then(|m| m.parse().ok())
                .unwrap_or(0),
        ) {
            // grouped by card image
            let rarities: IndexMap<_, _> = map
                .values()
                .filter(|c| c.card_number.eq_ignore_ascii_case(&self.card_number))
                .fold(Default::default(), |mut acc, c| {
                    acc.entry(&c.img).or_insert(c);
                    acc
                });
            rarities
                .keys()
                .enumerate()
                .filter(|(_, img)| ***img == c.img)
                .map(|(i, _)| i)
                .next()
                .unwrap_or_default() as u32
        } else {
            0
        }
    }

    pub fn to_lower_rarity(&self, map: &CardsInfoMap) -> Self {
        CommonCards::from_card_number(self.card_number.clone(), self.amount, map)
    }

    pub fn image_path(&self, map: &CardsInfoMap, lang: CardLanguage) -> Option<String> {
        let card = map.get(&self.manage_id.as_ref()?.parse::<u32>().ok()?)?;
        if lang == CardLanguage::English {
            if let Some(img_proxy_en) = &card.img_proxy_en {
                Some(format!(
                    "https://qrimpuff.github.io/hocg-fan-sim-assets/img_proxy_en/{img_proxy_en}",
                ))
            } else {
                // fallback to lower rarity
                let lower = self.to_lower_rarity(map);
                let card = map.get(&lower.manage_id.as_ref()?.parse::<u32>().ok()?)?;
                Some(format!(
                    "https://qrimpuff.github.io/hocg-fan-sim-assets/img_proxy_en/{}",
                    card.img_proxy_en.as_ref()?
                ))
            }
        } else {
            Some(format!(
                "https://qrimpuff.github.io/hocg-fan-sim-assets/img/{}",
                card.img
            ))
        }
    }
}

trait MergeCommonCards {
    fn merge(self) -> Self;
    fn merge_without_rarity(self) -> Self;
}
impl MergeCommonCards for Vec<CommonCards> {
    fn merge(self) -> Self {
        let mut map = IndexMap::with_capacity(self.len());

        for card in self {
            map.entry((card.card_number.clone(), card.manage_id.clone()))
                .and_modify(|c: &mut CommonCards| c.amount += card.amount)
                .or_insert(card);
        }

        map.into_values().collect()
    }

    fn merge_without_rarity(self) -> Self {
        let mut map = IndexMap::with_capacity(self.len());

        for card in self {
            map.entry(card.card_number.clone())
                .and_modify(|c: &mut CommonCards| c.amount += card.amount)
                .or_insert(card);
        }

        map.into_values().collect()
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
    pub fn all_cards(&self) -> impl Iterator<Item = &CommonCards> {
        std::iter::once(&self.oshi)
            .chain(self.main_deck.iter())
            .chain(self.cheer_deck.iter())
    }

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
    pub img_proxy_en: Option<String>,
    pub max: u32,
    pub deck_type: String,
}
