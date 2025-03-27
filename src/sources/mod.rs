use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use hocg_fan_sim_assets_model::{CardEntry, CardsInfo};
use indexmap::IndexMap;
use itertools::Itertools;
use price_check::PriceCache;
use web_time::Instant;

use crate::{CardLanguage, CardType};

pub mod deck_log;
pub mod edit_deck;
pub mod holodelta;
pub mod holoduel;
pub mod json;
pub mod price_check;
pub mod proxy_sheets;
pub mod starter_decks;
pub mod tabletop_sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckType {
    StarterDecks,
    DeckLog,
    HoloDelta,
    HoloDuel,
    TabletopSim,
    ProxySheets,
    PriceCheck,
    Unknown,
}

pub trait CommonCardConversion: Sized {
    type CardDeck;

    fn from_common_card(card: CommonCard, info: &CardsInfo) -> Self;
    fn to_common_card(value: Self, info: &CardsInfo) -> CommonCard;

    fn build_custom_deck(cards: Vec<CommonCard>, info: &CardsInfo) -> Self::CardDeck;
    fn build_common_deck(cards: Self::CardDeck, info: &CardsInfo) -> Vec<CommonCard>;
}

pub trait CommonDeckConversion {
    fn from_common_deck(deck: CommonDeck, info: &CardsInfo) -> Option<Self>
    where
        Self: Sized;
    fn to_common_deck(value: Self, info: &CardsInfo) -> CommonDeck;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommonCard {
    pub manage_id: Option<u32>,
    pub card_number: String,
    pub amount: u32,
}

impl CommonCard {
    pub fn from_manage_id(manage_id: u32, amount: u32, info: &CardsInfo) -> Self {
        let card = info
            .values()
            .flatten()
            .find(|c| c.manage_id == Some(manage_id));
        CommonCard {
            manage_id: card.and_then(|c| c.manage_id),
            card_number: card
                .map(|c| c.card_number.clone())
                .unwrap_or("UNKNOWN".into()),
            amount,
        }
    }

    pub fn from_card_number(card_number: String, amount: u32, info: &CardsInfo) -> Self {
        let card = info
            .values()
            .flatten()
            .find(|c| c.card_number.eq_ignore_ascii_case(&card_number));
        CommonCard {
            manage_id: card.and_then(|c| c.manage_id),
            card_number: card.map(|c| c.card_number.clone()).unwrap_or(card_number),
            amount,
        }
    }

    pub fn from_card_number_and_manage_id(
        card_number: String,
        manage_id: u32,
        amount: u32,
        info: &CardsInfo,
    ) -> Self {
        let card: Option<&CardEntry> = info
            .values()
            .flatten()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .find(|c| c.manage_id == Some(manage_id));
        if let Some(card) = card {
            CommonCard {
                manage_id: card.manage_id,
                card_number: card.card_number.clone(),
                amount,
            }
        } else {
            // default to basic rarity if not found
            CommonCard::from_card_number(card_number, amount, info)
        }
    }

    pub fn from_card_number_and_delta_art_index(
        card_number: String,
        delta_art_index: u32,
        amount: u32,
        info: &CardsInfo,
    ) -> Self {
        let card: Option<&CardEntry> = info
            .values()
            .flatten()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .find(|c| c.delta_art_index == Some(delta_art_index));
        if let Some(card) = card {
            CommonCard {
                manage_id: card.manage_id,
                card_number: card.card_number.clone(),
                amount,
            }
        } else {
            // default to basic rarity if not found
            CommonCard::from_card_number(card_number, amount, info)
        }
    }

    pub fn delta_art_index(&self, info: &CardsInfo) -> u32 {
        if let Some(c) = self.card_info(info) {
            if let Some(delta_art_index) = c.delta_art_index {
                return delta_art_index;
            }

            // fallback to a possible future art index
            let next_delta_art_index = info
                .values()
                .flatten()
                .filter(|c| c.card_number.eq_ignore_ascii_case(&self.card_number))
                .filter_map(|c| Some(c.delta_art_index? + 1))
                .max()
                .unwrap_or(0);
            next_delta_art_index
        } else {
            0
        }
    }

    pub fn to_lower_rarity(&self, info: &CardsInfo) -> Self {
        CommonCard::from_card_number(self.card_number.clone(), self.amount, info)
    }

    pub fn card_info<'a>(&self, info: &'a CardsInfo) -> Option<&'a CardEntry> {
        info.get(&self.card_number)
            .into_iter()
            .flatten()
            .find(|c| c.manage_id == self.manage_id)
    }

    pub fn card_type(&self, info: &CardsInfo) -> Option<CardType> {
        match self.card_info(info).map(|c| c.deck_type.as_str()) {
            Some("OSHI") => Some(CardType::Oshi),
            Some("YELL") => Some(CardType::Cheer),
            Some("N") => Some(CardType::Main),
            _ => None,
        }
    }

    pub fn price(&self, info: &CardsInfo, prices: &PriceCache) -> Option<u32> {
        self.price_cache(info, prices).map(|p| p.1)
    }
    pub fn price_cache<'a>(
        &self,
        info: &CardsInfo,
        prices: &'a PriceCache,
    ) -> Option<&'a (Instant, u32)> {
        self.card_info(info)
            .and_then(|c| c.yuyutei_sell_url.as_ref())
            .and_then(|y| prices.get(y))
    }

    pub fn alt_cards(&self, info: &CardsInfo) -> Vec<Self> {
        info.values()
            .flatten()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&self.card_number))
            .map(|c| Self {
                manage_id: c.manage_id,
                card_number: c.card_number.clone(),
                amount: self.amount,
            })
            .collect_vec()
    }

    pub fn image_path(&self, info: &CardsInfo, lang: CardLanguage) -> Option<String> {
        let card = self.card_info(info)?;
        if lang == CardLanguage::English {
            if let Some(img_proxy_en) = &card.img_proxy_en {
                Some(format!(
                    "https://qrimpuff.github.io/hocg-fan-sim-assets/img_proxy_en/{img_proxy_en}",
                ))
            } else {
                // fallback to lower rarity
                let lower = self.to_lower_rarity(info);
                let card = lower.card_info(info)?;
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
impl MergeCommonCards for Vec<CommonCard> {
    fn merge(self) -> Self {
        let mut map = IndexMap::with_capacity(self.len());

        for card in self {
            // skip cards with 0 amount
            if card.amount == 0 {
                continue;
            }

            // merge cards with the same manage_id
            map.entry((card.card_number.clone(), card.manage_id))
                .and_modify(|c: &mut CommonCard| c.amount += card.amount)
                .or_insert(card);
        }

        map.into_values().collect()
    }

    fn merge_without_rarity(self) -> Self {
        let mut map = IndexMap::with_capacity(self.len());

        for card in self {
            map.entry(card.card_number.clone())
                .and_modify(|c: &mut CommonCard| c.amount += card.amount)
                .or_insert(card);
        }

        map.into_values().collect()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
/// Is a partial representation of a deck, used for editing and importing/exporting
pub struct CommonDeck {
    pub name: Option<String>,
    pub oshi: Option<CommonCard>,
    pub main_deck: Vec<CommonCard>,
    pub cheer_deck: Vec<CommonCard>,
}

impl CommonDeck {
    pub fn all_cards(&self) -> impl Iterator<Item = &CommonCard> {
        self.oshi
            .iter()
            .chain(self.main_deck.iter())
            .chain(self.cheer_deck.iter())
    }

    pub fn all_cards_mut(&mut self) -> impl Iterator<Item = &mut CommonCard> {
        self.oshi
            .iter_mut()
            .chain(self.main_deck.iter_mut())
            .chain(self.cheer_deck.iter_mut())
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
        if let Some(oshi) = &self.oshi {
            // TODO use card name
            // format!("{}'s deck", oshi.card_number)
            format!("Custom deck - {}", oshi.card_number)
        } else {
            "Custom deck".into()
        }
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

    pub fn merge(&mut self) {
        // remove oshi card if amount is 0
        if let Some(oshi) = &self.oshi {
            if oshi.amount == 0 {
                self.oshi = None;
            }
        }
        self.main_deck = std::mem::take(&mut self.main_deck).merge();
        self.cheer_deck = std::mem::take(&mut self.cheer_deck).merge();
    }

    pub fn is_empty(&self) -> bool {
        self.oshi.is_none() && self.main_deck.is_empty() && self.cheer_deck.is_empty()
    }

    pub fn validate(&self, info: &CardsInfo) -> Vec<String> {
        let mut errors = vec![];

        // check for unreleased or invalid cards
        if self.oshi.iter().any(|c| c.manage_id.is_none())
            || self.main_deck.iter().any(|c| c.manage_id.is_none())
            || self.cheer_deck.iter().any(|c| c.manage_id.is_none())
        {
            errors.push("Contains unknown cards.".into());
        }

        // check for card amount
        let oshi_amount = self.oshi.iter().map(|c| c.amount).sum::<u32>();
        if oshi_amount > 1 {
            errors.push("Too many Oshi cards.".to_string());
        }
        if oshi_amount < 1 {
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
            .map(|(k, v)| CommonCard::from_card_number(k.clone(), v, info))
        {
            let max = card.card_info(info).map(|i| i.max).unwrap_or(50);
            if card.amount > max {
                errors.push(format!(
                    "Too many {} in main deck. ({} cards; {max} max)",
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

    pub fn find_card(&self, manage_id: u32) -> Option<&CommonCard> {
        self.all_cards().find(|c| c.manage_id == Some(manage_id))
    }

    pub fn add_card(&mut self, card: CommonCard, card_type: CardType, info: &CardsInfo) {
        match card.card_type(info).unwrap_or(card_type) {
            CardType::Oshi => self.oshi = Some(card.clone()),
            CardType::Main => self.main_deck.push(card),
            CardType::Cheer => self.cheer_deck.push(card),
        }
        self.merge();
    }

    pub fn remove_card(&mut self, card: CommonCard, card_type: CardType, info: &CardsInfo) {
        match card.card_type(info).unwrap_or(card_type) {
            CardType::Oshi => self.oshi.iter_mut().for_each(|c| {
                if c.manage_id == card.manage_id && c.card_number == card.card_number {
                    c.amount = c.amount.saturating_sub(card.amount);
                }
            }),
            CardType::Main => self.main_deck.iter_mut().for_each(|c| {
                if c.manage_id == card.manage_id && c.card_number == card.card_number {
                    c.amount = c.amount.saturating_sub(card.amount);
                }
            }),
            CardType::Cheer => self.cheer_deck.iter_mut().for_each(|c| {
                if c.manage_id == card.manage_id && c.card_number == card.card_number {
                    c.amount = c.amount.saturating_sub(card.amount);
                }
            }),
        }
        self.merge();
    }
}
