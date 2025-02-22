use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use hocg_fan_sim_assets_model::{CardEntry, CardsInfo};
use indexmap::IndexMap;
use itertools::Itertools;
use price_check::PriceCache;
use web_time::Instant;

use crate::CardLanguage;

pub mod deck_log;
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

pub trait CommonCardsConversion: Sized {
    type CardDeck;

    fn from_common_cards(cards: CommonCards, info: &CardsInfo) -> Self;
    fn to_common_cards(value: Self, info: &CardsInfo) -> CommonCards;

    fn build_custom_deck(cards: Vec<CommonCards>, info: &CardsInfo) -> Self::CardDeck;
    fn build_common_deck(cards: Self::CardDeck, info: &CardsInfo) -> Vec<CommonCards>;
}

pub trait CommonDeckConversion {
    fn from_common_deck(deck: CommonDeck, info: &CardsInfo) -> Self;
    fn to_common_deck(value: Self, info: &CardsInfo) -> CommonDeck;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommonCards {
    pub manage_id: Option<u32>,
    pub card_number: String,
    pub amount: u32,
}

impl CommonCards {
    pub fn from_manage_id(manage_id: u32, amount: u32, info: &CardsInfo) -> Self {
        let card = info
            .values()
            .flatten()
            .find(|c| c.manage_id == Some(manage_id));
        CommonCards {
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
        CommonCards {
            manage_id: card.and_then(|c| c.manage_id),
            card_number: card.map(|c| c.card_number.clone()).unwrap_or(card_number),
            amount,
        }
    }

    pub fn from_card_number_and_index(
        card_number: String,
        rarity_index: u32,
        amount: u32,
        info: &CardsInfo,
    ) -> Self {
        // grouped by card image
        let rarities: IndexMap<_, _> = info
            .values()
            .flatten()
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .fold(Default::default(), |mut acc, c| {
                acc.entry(&c.img).or_insert(c);
                acc
            });
        let card = rarities.values().nth(rarity_index as usize);
        if let Some(card) = card {
            CommonCards {
                manage_id: card.manage_id,
                card_number: card.card_number.clone(),
                amount,
            }
        } else {
            // default to basic rarity if not found
            CommonCards::from_card_number(card_number, amount, info)
        }
    }

    pub fn rarity_index(&self, info: &CardsInfo) -> u32 {
        if let Some(c) = self.card_info(info) {
            // grouped by card image
            let rarities: IndexMap<_, _> = info
                .values()
                .flatten()
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

    pub fn to_lower_rarity(&self, info: &CardsInfo) -> Self {
        CommonCards::from_card_number(self.card_number.clone(), self.amount, info)
    }

    pub fn card_info<'a>(&self, info: &'a CardsInfo) -> Option<&'a CardEntry> {
        info.get(&self.card_number)
            .into_iter()
            .flatten()
            .find(|c| c.manage_id == self.manage_id)
    }

    pub fn card_info_mut<'a>(&self, info: &'a mut CardsInfo) -> Option<&'a mut CardEntry> {
        info.get_mut(&self.card_number)
            .into_iter()
            .flatten()
            .find(|c| c.manage_id == self.manage_id)
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
impl MergeCommonCards for Vec<CommonCards> {
    fn merge(self) -> Self {
        let mut map = IndexMap::with_capacity(self.len());

        for card in self {
            map.entry((card.card_number.clone(), card.manage_id))
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

    pub fn all_cards_mut(&mut self) -> impl Iterator<Item = &mut CommonCards> {
        std::iter::once(&mut self.oshi)
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

    pub fn merge(self) -> Self {
        CommonDeck {
            name: self.name,
            oshi: self.oshi,
            main_deck: self.main_deck.merge(),
            cheer_deck: self.cheer_deck.merge(),
        }
    }

    pub fn validate(&self, info: &CardsInfo) -> Vec<String> {
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
            .map(|(k, v)| CommonCards::from_card_number(k.clone(), v, info))
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
}

// // need to keep the order to know which card image to use
// // (holoDelta is using a zero-based index)
// pub type CardsInfo = BTreeMap<u32, CardEntry>;

// #[derive(Debug, Clone, Deserialize)]
// #[serde(rename_all = "snake_case")]
// // from https://qrimpuff.github.io/hocg-fan-sim-assets/cards_info.json
// pub struct CardEntry {
//     pub manage_id: String,
//     pub card_number: String,
//     pub rare: String,
//     pub img: String,
//     pub img_proxy_en: Option<String>,
//     pub max: u32,
//     pub deck_type: String,
//     pub yuyutei_sell_url: Option<String>,
//     #[serde(skip)]
//     // used to cache the price after lookup
//     pub price_yen: Option<(Instant, u32)>,
// }
