use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
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

    fn from_common_card(card: CommonCard, db: &CardsDatabase) -> Self;
    fn to_common_card(value: Self, db: &CardsDatabase) -> CommonCard;

    fn build_custom_deck(cards: Vec<CommonCard>, db: &CardsDatabase) -> Self::CardDeck;
    fn build_common_deck(cards: Self::CardDeck, db: &CardsDatabase) -> Vec<CommonCard>;
}

pub trait CommonDeckConversion {
    fn from_common_deck(deck: CommonDeck, db: &CardsDatabase) -> Option<Self>
    where
        Self: Sized;
    fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommonCard {
    pub manage_id: Option<u32>,
    pub card_number: String,
    pub amount: u32,
}

impl CommonCard {
    pub fn from_manage_id(manage_id: u32, amount: u32, db: &CardsDatabase) -> Self {
        let card = db
            .values()
            .flat_map(|c| &c.illustrations)
            .find(|c| c.manage_id == Some(manage_id));
        CommonCard {
            manage_id: card.and_then(|c| c.manage_id),
            card_number: card
                .map(|c| c.card_number.clone())
                .unwrap_or("UNKNOWN".into()),
            amount,
        }
    }

    pub fn from_card_number(card_number: String, amount: u32, db: &CardsDatabase) -> Self {
        let card = db
            .values()
            .flat_map(|c| &c.illustrations)
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
        db: &CardsDatabase,
    ) -> Self {
        let card: Option<_> = db
            .values()
            .flat_map(|c| &c.illustrations)
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
            CommonCard::from_card_number(card_number, amount, db)
        }
    }

    pub fn from_card_number_and_delta_art_index(
        card_number: String,
        delta_art_index: u32,
        amount: u32,
        db: &CardsDatabase,
    ) -> Self {
        let card: Option<_> = db
            .values()
            .flat_map(|c| &c.illustrations)
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
            CommonCard::from_card_number(card_number, amount, db)
        }
    }

    pub fn delta_art_index(&self, db: &CardsDatabase) -> u32 {
        if let Some(c) = self.card_illustration(db) {
            if let Some(delta_art_index) = c.delta_art_index {
                return delta_art_index;
            }

            // fallback to a possible future art index
            let next_delta_art_index = db
                .values()
                .flat_map(|c| &c.illustrations)
                .filter(|c| c.card_number.eq_ignore_ascii_case(&self.card_number))
                .filter_map(|c| Some(c.delta_art_index? + 1))
                .max()
                .unwrap_or(0);
            next_delta_art_index
        } else {
            0
        }
    }

    pub fn to_lower_rarity(&self, db: &CardsDatabase) -> Self {
        CommonCard::from_card_number(self.card_number.clone(), self.amount, db)
    }

    pub fn card_info<'a>(&self, db: &'a CardsDatabase) -> Option<&'a hocg::Card> {
        db.get(&self.card_number)
    }

    pub fn card_illustration<'a>(
        &self,
        db: &'a CardsDatabase,
    ) -> Option<&'a hocg::CardIllustration> {
        self.card_info(db).and_then(|c| {
            c.illustrations
                .iter()
                .find(|i| i.manage_id == self.manage_id)
        })
    }

    pub fn card_type(&self, db: &CardsDatabase) -> Option<CardType> {
        match self.card_info(db).map(|c| c.card_type) {
            Some(hocg::CardType::OshiHoloMember) => Some(CardType::Oshi),
            Some(hocg::CardType::Cheer) => Some(CardType::Cheer),
            Some(_) => Some(CardType::Main),
            _ => None,
        }
    }

    pub fn price(&self, db: &CardsDatabase, prices: &PriceCache) -> Option<u32> {
        self.price_cache(db, prices).map(|p| p.1)
    }
    pub fn price_cache<'a>(
        &self,
        db: &CardsDatabase,
        prices: &'a PriceCache,
    ) -> Option<&'a (Instant, u32)> {
        self.card_illustration(db)
            .and_then(|c| c.yuyutei_sell_url.as_ref())
            .and_then(|y| prices.get(y))
    }

    pub fn alt_cards(&self, db: &CardsDatabase) -> Vec<Self> {
        db.values()
            .flat_map(|c| &c.illustrations)
            .filter(|c| {
                if self.card_type(db) == Some(CardType::Cheer) {
                    // all cheers of the same color are considered alt cards. e.g. hY01-001 = hY01-002
                    c.card_number.split_once('-').map(|n| n.0)
                        == self.card_number.split_once('-').map(|n| n.0)
                } else {
                    c.card_number.eq_ignore_ascii_case(&self.card_number)
                }
            })
            .map(|c| Self {
                manage_id: c.manage_id,
                card_number: c.card_number.clone(),
                amount: self.amount,
            })
            .collect_vec()
    }

    pub fn image_path(
        &self,
        db: &CardsDatabase,
        lang: CardLanguage,
        fallback_rarity: bool,
        fallback_lang: bool,
    ) -> Option<String> {
        let card = self.card_illustration(db)?;
        if lang == CardLanguage::English {
            // exact match for english cards
            if let Some(img_en) = &card.img_path.english {
                return Some(format!(
                    "https://qrimpuff.github.io/hocg-fan-sim-assets/img_en/{img_en}",
                ));
            }

            // fallback to similar card images
            if let Some(img_en) = self
                .card_info(db)
                .iter()
                .flat_map(|c| &c.illustrations)
                .find(|i| i.delta_art_index == card.delta_art_index && i.img_path.english.is_some())
                .and_then(|i| i.img_path.english.as_ref())
            {
                return Some(format!(
                    "https://qrimpuff.github.io/hocg-fan-sim-assets/img_en/{img_en}",
                ));
            }

            // fallback to lower rarity
            if fallback_rarity {
                let lower = self.to_lower_rarity(db);
                if lower != *self {
                    return lower.image_path(db, lang, false, fallback_lang);
                }
            }

            if !fallback_lang {
                return None;
            }
        }

        // we always have a japanese image
        Some(format!(
            "https://qrimpuff.github.io/hocg-fan-sim-assets/img/{}",
            card.img_path.japanese
        ))
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

    pub fn required_deck_name(&self, db: &CardsDatabase) -> String {
        self.required_deck_name_max_length(usize::MAX, db)
    }

    pub fn required_deck_name_max_length(&self, max_length: usize, db: &CardsDatabase) -> String {
        if let Some(name) = self
            .name
            .as_ref()
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
        {
            name.to_string()
        } else {
            self.default_deck_name(max_length, db)
        }
    }

    fn default_deck_name(&self, max_length: usize, db: &CardsDatabase) -> String {
        if let Some(oshi) = &self.oshi {
            if let Some(oshi) = oshi.card_info(db) {
                let name = oshi
                    .name
                    .english
                    .as_ref()
                    .unwrap_or(&oshi.name.japanese)
                    .to_string();
                let name = format!("Custom deck - {}", name);
                if name.len() <= max_length {
                    return name;
                }
            }

            let name = format!("Custom deck - {}", oshi.card_number);
            if name.len() <= max_length {
                return name;
            }
        }

        "Custom deck".into()
    }

    pub fn file_name(&self, db: &CardsDatabase) -> String {
        let mut name = self.required_deck_name(db);
        if !name.is_ascii() {
            name = "Custom deck".into();
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

    pub fn validate(&self, db: &CardsDatabase) -> Vec<String> {
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
            .map(|(k, v)| CommonCard::from_card_number(k.clone(), v, db))
        {
            let max = card.card_info(db).map(|i| i.max_amount).unwrap_or(50);
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

    pub fn add_card(&mut self, card: CommonCard, card_type: CardType, db: &CardsDatabase) {
        match card.card_type(db).unwrap_or(card_type) {
            CardType::Oshi => self.oshi = Some(card.clone()),
            CardType::Main => self.main_deck.push(card),
            CardType::Cheer => self.cheer_deck.push(card),
        }
        self.merge();

        // sort the decks
        self.main_deck
            .sort_by_cached_key(|c| (c.card_info(db), c.manage_id));
        self.cheer_deck
            .sort_by_cached_key(|c| (c.card_info(db), c.manage_id));
    }

    pub fn remove_card(&mut self, card: CommonCard, card_type: CardType, db: &CardsDatabase) {
        match card.card_type(db).unwrap_or(card_type) {
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
