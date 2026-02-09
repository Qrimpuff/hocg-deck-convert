use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
};

use hocg_fan_sim_assets_model::{self as hocg, CardIllustration, CardsDatabase};
use icu::decimal::{DecimalFormatter, input::Decimal};
use icu::locale::locale;
use indexmap::IndexMap;
use itertools::Itertools;
use jiff::Timestamp;
use price_check::PriceCache;
use serde::{Deserialize, Serialize};

use crate::{
    CardLanguage, CardType,
    sources::price_check::{PriceCacheKey, PriceCheckService},
};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommonCard {
    pub card_number: String,
    pub illustration_idx: Option<usize>,
    pub amount: u32,
}

impl CommonCard {
    fn from_card_number_and_illustration_idx(
        card_number: String,
        illustration_idx: usize,
        amount: u32,
    ) -> Self {
        CommonCard {
            illustration_idx: Some(illustration_idx),
            card_number,
            amount,
        }
    }

    pub fn from_card_illustration(
        card: &CardIllustration,
        amount: u32,
        db: &CardsDatabase,
    ) -> Self {
        let found: Option<_> = db
            // this is a clean card number, will be a valid key in the database
            .get(&card.card_number)
            .and_then(|c| c.illustrations.iter().enumerate().find(|(_, c)| *c == card));
        if let Some((idx, card)) = found {
            CommonCard::from_card_number_and_illustration_idx(card.card_number.clone(), idx, amount)
        } else {
            CommonCard {
                card_number: card.card_number.clone(),
                illustration_idx: None,
                amount,
            }
        }
    }

    pub fn from_manage_id(manage_id: (CardLanguage, u32), amount: u32, db: &CardsDatabase) -> Self {
        let found: Option<_> = db
            .values()
            .flat_map(|c| c.illustrations.iter().enumerate())
            .find(|(_, c)| {
                c.manage_id
                    .value(manage_id.0.into())
                    .iter()
                    .flatten()
                    .any(|m| *m == manage_id.1)
            });
        if let Some((idx, card)) = found {
            CommonCard::from_card_number_and_illustration_idx(card.card_number.clone(), idx, amount)
        } else {
            CommonCard {
                card_number: "UNKNOWN".into(),
                illustration_idx: None,
                amount,
            }
        }
    }

    pub fn from_card_number(card_number: String, amount: u32, db: &CardsDatabase) -> Self {
        let found: Option<_> = db
            .values()
            // this card number could be in any case
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .flat_map(|c| c.illustrations.iter().enumerate())
            .next();
        if let Some((idx, card)) = found {
            CommonCard::from_card_number_and_illustration_idx(card.card_number.clone(), idx, amount)
        } else {
            CommonCard {
                card_number,
                illustration_idx: None,
                amount,
            }
        }
    }

    pub fn from_card_number_and_manage_id(
        card_number: String,
        manage_id: (CardLanguage, u32),
        amount: u32,
        db: &CardsDatabase,
    ) -> Self {
        let found: Option<_> = db
            .values()
            // this card number could be in any case
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .flat_map(|c| c.illustrations.iter().enumerate())
            .find(|(_, c)| {
                c.manage_id
                    .value(manage_id.0.into())
                    .iter()
                    .flatten()
                    .any(|m| *m == manage_id.1)
            });
        if let Some((idx, card)) = found {
            CommonCard::from_card_number_and_illustration_idx(card.card_number.clone(), idx, amount)
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
        let found: Option<_> = db
            .values()
            // this card number could be in any case
            .filter(|c| c.card_number.eq_ignore_ascii_case(&card_number))
            .flat_map(|c| c.illustrations.iter().enumerate())
            .find(|(_, c)| c.delta_art_index == Some(delta_art_index));
        if let Some((idx, card)) = found {
            CommonCard::from_card_number_and_illustration_idx(card.card_number.clone(), idx, amount)
        } else {
            // default to basic rarity if not found
            CommonCard::from_card_number(card_number, amount, db)
        }
    }

    pub fn first_manage_id(&self, language: CardLanguage, db: &CardsDatabase) -> Option<u32> {
        self.card_illustration(db)?
            .manage_id
            .value(language.into())
            .iter()
            .flatten()
            .copied()
            .next()
    }

    pub fn delta_art_index(&self, db: &CardsDatabase) -> u32 {
        if let Some(c) = self.card_illustration(db) {
            if let Some(delta_art_index) = c.delta_art_index {
                return delta_art_index;
            }

            // fallback to a possible future art index
            db.values()
                .flat_map(|c| &c.illustrations)
                .filter(|c| c.card_number.eq_ignore_ascii_case(&self.card_number))
                .filter_map(|c| Some(c.delta_art_index? + 1))
                .max()
                .unwrap_or(0)
        } else {
            0
        }
    }

    pub fn to_lower_rarity(
        &self,
        db: &CardsDatabase,
        language: CardLanguage,
        allow_proxy: bool,
    ) -> Self {
        let found = db.get(&self.card_number).and_then(|c| {
            c.illustrations
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    allow_proxy
                        || c.img_path
                            .value(language.into())
                            .as_ref()
                            .is_some_and(|path| !path.contains("proxies"))
                })
                .find(|(_, c)| c.img_path.value(language.into()).is_some())
        });
        if let Some((idx, card)) = found {
            CommonCard::from_card_number_and_illustration_idx(
                card.card_number.clone(),
                idx,
                self.amount,
            )
        } else {
            self.clone()
        }
    }

    pub fn card_info<'a>(&self, db: &'a CardsDatabase) -> Option<&'a hocg::Card> {
        db.get(&self.card_number)
    }

    pub fn card_illustration<'a>(
        &self,
        db: &'a CardsDatabase,
    ) -> Option<&'a hocg::CardIllustration> {
        self.card_info(db)
            .and_then(|c| c.illustrations.get(self.illustration_idx?))
    }

    pub fn card_type(&self, db: &CardsDatabase) -> Option<CardType> {
        match self.card_info(db).map(|c| c.card_type) {
            Some(hocg::CardType::OshiHoloMember) => Some(CardType::Oshi),
            Some(hocg::CardType::Cheer) => Some(CardType::Cheer),
            Some(_) => Some(CardType::Main),
            _ => None,
        }
    }

    pub fn is_basic_cheer(&self) -> bool {
        matches!(
            self.card_number.as_str(),
            "hY01-001" | "hY02-001" | "hY03-001" | "hY04-001" | "hY05-001" | "hY06-001"
        )
    }

    pub fn price(
        &self,
        db: &CardsDatabase,
        prices: &PriceCache,
        service: PriceCheckService,
        free_basic_cheers: bool,
    ) -> Option<f64> {
        if free_basic_cheers && self.is_basic_cheer() {
            Some(0.0)
        } else {
            self.price_cache(db, prices, service).map(|p| p.1)
        }
    }
    pub fn price_display(
        &self,
        db: &CardsDatabase,
        prices: &PriceCache,
        service: PriceCheckService,
        free_basic_cheers: bool,
    ) -> Option<String> {
        self.price(db, prices, service, free_basic_cheers)
            .map(|p| match service {
                PriceCheckService::Yuyutei => {
                    let f = DecimalFormatter::try_new(locale!("ja-JP").into(), Default::default())
                        .expect("locale should be present");
                    let p = Decimal::from_str(format!("{p}").as_str()).unwrap();
                    let p = f.format(&p);
                    format!("¥{p}")
                }
                PriceCheckService::TcgPlayer => {
                    let f = DecimalFormatter::try_new(locale!("en-US").into(), Default::default())
                        .expect("locale should be present");
                    let p = Decimal::from_str(format!("{p:.2}").as_str()).unwrap();
                    let p = f.format(&p);
                    format!("${p}")
                }
            })
    }
    pub fn price_url(&self, db: &CardsDatabase, service: PriceCheckService) -> Option<String> {
        self.card_illustration(db).and_then(|c| match service {
            PriceCheckService::Yuyutei => c.yuyutei_sell_url.clone(),
            PriceCheckService::TcgPlayer => c.tcgplayer_url(),
        })
    }
    pub fn price_cache<'a>(
        &self,
        db: &CardsDatabase,
        prices: &'a PriceCache,
        service: PriceCheckService,
    ) -> Option<&'a (Timestamp, f64)> {
        self.card_illustration(db).and_then(|c| {
            prices.get(&match service {
                PriceCheckService::Yuyutei => {
                    PriceCacheKey::Yuyutei(c.yuyutei_sell_url.as_ref()?.to_string())
                }
                PriceCheckService::TcgPlayer => PriceCacheKey::TcgPlayer(c.tcgplayer_product_id?),
            })
        })
    }

    pub fn alt_cards(&self, db: &CardsDatabase) -> Vec<Self> {
        let is_cheer = self.card_type(db) == Some(CardType::Cheer);
        db.values()
            .filter(|c| {
                if is_cheer {
                    // all cheers of the same color are considered alt cards. e.g. hY01-001 = hY01-002
                    c.card_number.split_once('-').map(|n| n.0)
                        == self.card_number.split_once('-').map(|n| n.0)
                } else {
                    c.card_number.eq_ignore_ascii_case(&self.card_number)
                }
            })
            .flat_map(|c| c.illustrations.iter().enumerate())
            .map(|(idx, c)| {
                CommonCard::from_card_number_and_illustration_idx(
                    c.card_number.clone(),
                    idx,
                    self.amount,
                )
            })
            .collect_vec()
    }

    pub fn image_path(
        &self,
        db: &CardsDatabase,
        language: CardLanguage,
        opts: ImageOptions,
    ) -> Option<String> {
        let card = self.card_illustration(db)?;

        let assets_url = match language {
            CardLanguage::Japanese => "https://qrimpuff.github.io/hocg-fan-sim-assets/img/",
            CardLanguage::English => "https://qrimpuff.github.io/hocg-fan-sim-assets/img_en/",
        };

        // exact match first
        if (opts.allow_proxy
            || card
                .img_path
                .value(language.into())
                .as_ref()
                .is_some_and(|path| !path.contains("proxies")))
            && let Some(img) = card.img_path.value(language.into()).as_ref()
        {
            return Some(format!("{assets_url}{img}"));
        }

        // fallback to similar card images
        if opts.fallback_similar
            && let Some(img) = self
                .card_info(db)
                .iter()
                .flat_map(|c| &c.illustrations)
                .filter(|i| {
                    opts.allow_proxy
                        || i.img_path
                            .value(language.into())
                            .as_ref()
                            .is_some_and(|path| !path.contains("proxies"))
                })
                .find(|i| {
                    i.delta_art_index.is_some()
                        && i.delta_art_index == card.delta_art_index
                        && i.img_path.value(language.into()).is_some()
                })
                .and_then(|i| i.img_path.value(language.into()).as_ref())
        {
            return Some(format!("{assets_url}{img}"));
        }

        // fallback to lower rarity
        if opts.fallback_rarity {
            let lower = self.to_lower_rarity(db, language, opts.allow_proxy);
            if lower != *self {
                return lower.image_path(
                    db,
                    language,
                    ImageOptions {
                        fallback_rarity: false,
                        ..opts
                    },
                );
            }
        }

        // fallback to another language
        if opts.fallback_lang {
            let other_language = match language {
                CardLanguage::Japanese => CardLanguage::English,
                CardLanguage::English => CardLanguage::Japanese,
            };
            return self.image_path(
                db,
                other_language,
                ImageOptions {
                    fallback_lang: false,
                    ..opts
                },
            );
        }

        // no image found
        None
    }

    pub fn is_unknown(&self, db: &CardsDatabase) -> bool {
        self.card_info(db).is_none()
    }

    pub fn is_unreleased(&self, language: CardLanguage, db: &CardsDatabase) -> bool {
        // does not overlap with `is_unknown`
        !self.is_unknown(db)
            && self
                .card_illustration(db)
                .is_none_or(|c| c.manage_id.value(language.into()).is_none())
    }

    pub fn max_amount(&self, language: CardLanguage, db: &CardsDatabase) -> u32 {
        self.card_info(db)
            .and_then(|i| {
                i.max_amount
                    .value(language.into())
                    .or_else(|| *i.max_amount.value(CardLanguage::Japanese.into()))
                    .or_else(|| *i.max_amount.value(CardLanguage::English.into()))
            })
            .unwrap_or(50)
    }

    pub fn html_id(&self) -> String {
        format!(
            "card_{}_{}",
            self.card_number.to_lowercase(),
            match self.illustration_idx {
                Some(illustration_idx) => format!("{illustration_idx}"),
                None => "unknown".to_string(),
            }
        )
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct ImageOptions {
    pub fallback_similar: bool,
    pub fallback_rarity: bool,
    pub fallback_lang: bool,
    pub allow_proxy: bool,
}

impl ImageOptions {
    pub fn card_details() -> Self {
        ImageOptions {
            fallback_similar: false,
            fallback_rarity: false,
            fallback_lang: true,
            allow_proxy: true,
        }
    }

    pub fn card_search() -> Self {
        ImageOptions {
            fallback_similar: true,
            fallback_rarity: false,
            fallback_lang: true,
            allow_proxy: false,
        }
    }

    pub fn proxy_validation() -> Self {
        ImageOptions {
            fallback_similar: true,
            fallback_rarity: true,
            fallback_lang: false,
            allow_proxy: true,
        }
    }

    pub fn proxy_print() -> Self {
        ImageOptions {
            fallback_similar: true,
            fallback_rarity: true,
            fallback_lang: true,
            allow_proxy: true,
        }
    }

    pub fn deck_log() -> Self {
        ImageOptions {
            fallback_similar: false,
            fallback_rarity: false,
            fallback_lang: true,
            allow_proxy: false,
        }
    }

    pub fn holodelta() -> Self {
        ImageOptions {
            fallback_similar: true,
            fallback_rarity: true,
            fallback_lang: true,
            allow_proxy: true,
        }
    }

    pub fn price_check() -> Self {
        ImageOptions {
            fallback_similar: false,
            fallback_rarity: false,
            fallback_lang: false,
            allow_proxy: false,
        }
    }
}

trait MergeCommonCards {
    fn merge(self) -> Self;
    fn merge_delta(self, db: &CardsDatabase) -> Self;
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

            // merge cards with the same illustration_idx
            map.entry((card.card_number.clone(), card.illustration_idx))
                .and_modify(|c: &mut CommonCard| c.amount += card.amount)
                .or_insert(card);
        }

        map.into_values().collect()
    }

    fn merge_delta(self, db: &CardsDatabase) -> Self {
        let mut map = IndexMap::with_capacity(self.len());

        for card in self {
            // merge cards with the same delta_art_index
            map.entry((card.card_number.clone(), card.delta_art_index(db)))
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
                    .as_deref()
                    .and(oshi.name.japanese.as_deref())
                    .unwrap_or("Unknown")
                    .to_string();
                let name = format!("Custom deck - {name}");
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
        if let Some(oshi) = &self.oshi
            && oshi.amount == 0
        {
            self.oshi = None;
        }
        self.main_deck = std::mem::take(&mut self.main_deck).merge();
        self.cheer_deck = std::mem::take(&mut self.cheer_deck).merge();
    }

    pub fn is_empty(&self) -> bool {
        self.oshi.is_none() && self.main_deck.is_empty() && self.cheer_deck.is_empty()
    }

    pub fn validate(
        &self,
        db: &CardsDatabase,
        allow_unreleased: bool,
        language: CardLanguage,
    ) -> Vec<String> {
        let mut errors = vec![];

        // check for unreleased or invalid cards
        if self.oshi.iter().any(|c| c.is_unknown(db))
            || self.main_deck.iter().any(|c| c.is_unknown(db))
            || self.cheer_deck.iter().any(|c| c.is_unknown(db))
        {
            errors.push("Contains unknown cards.".into());
        }
        if !allow_unreleased
            && (self.oshi.iter().any(|c| c.is_unreleased(language, db))
                || self.main_deck.iter().any(|c| c.is_unreleased(language, db))
                || self
                    .cheer_deck
                    .iter()
                    .any(|c| c.is_unreleased(language, db)))
        {
            errors.push("Contains unreleased cards.".into());
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
            let max = card.max_amount(language, db);
            if card.amount > max {
                errors.push(format!(
                    "Too many {} in deck. ({} cards; {max} max for {})",
                    card.card_number,
                    card.amount,
                    match language {
                        CardLanguage::Japanese => "JP",
                        CardLanguage::English => "EN",
                    }
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

    pub fn card_amount(&self, card_number: &str, illustration_idx: Option<usize>) -> u32 {
        self.all_cards()
            .find(|c| c.card_number == card_number && c.illustration_idx == illustration_idx)
            .map_or(0, |c| c.amount)
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
            .sort_by_cached_key(|c| (c.card_info(db), c.illustration_idx));
        self.cheer_deck
            .sort_by_cached_key(|c| (c.card_info(db), c.illustration_idx));
    }

    pub fn remove_card(&mut self, card: CommonCard, card_type: CardType, db: &CardsDatabase) {
        match card.card_type(db).unwrap_or(card_type) {
            CardType::Oshi => self.oshi.iter_mut().for_each(|c| {
                if c.illustration_idx == card.illustration_idx && c.card_number == card.card_number
                {
                    c.amount = c.amount.saturating_sub(card.amount);
                }
            }),
            CardType::Main => self.main_deck.iter_mut().for_each(|c| {
                if c.illustration_idx == card.illustration_idx && c.card_number == card.card_number
                {
                    c.amount = c.amount.saturating_sub(card.amount);
                }
            }),
            CardType::Cheer => self.cheer_deck.iter_mut().for_each(|c| {
                if c.illustration_idx == card.illustration_idx && c.card_number == card.card_number
                {
                    c.amount = c.amount.saturating_sub(card.amount);
                }
            }),
        }
        self.merge();
    }

    pub fn price(
        &self,
        db: &CardsDatabase,
        prices: &PriceCache,
        service: PriceCheckService,
        free_basic_cheers: bool,
    ) -> f64 {
        self.all_cards()
            .filter_map(|c| {
                c.price(db, prices, service, free_basic_cheers)
                    .map(|p| (c, p))
            })
            .map(|(c, p)| p * c.amount as f64)
            .sum()
    }
    pub fn is_price_approximate(
        &self,
        db: &CardsDatabase,
        prices: &PriceCache,
        service: PriceCheckService,
        free_basic_cheers: bool,
    ) -> bool {
        self.all_cards()
            .any(|c| c.price(db, prices, service, free_basic_cheers).is_none())
    }
    pub fn price_display(
        &self,
        db: &CardsDatabase,
        prices: &PriceCache,
        service: PriceCheckService,
        free_basic_cheers: bool,
    ) -> String {
        let approx_price = if self.is_price_approximate(db, prices, service, free_basic_cheers) {
            ">"
        } else {
            ""
        };
        let price = self.price(db, prices, service, free_basic_cheers);
        let price = match service {
            PriceCheckService::Yuyutei => {
                let f = DecimalFormatter::try_new(locale!("ja-JP").into(), Default::default())
                    .expect("locale should be present");
                let p = Decimal::from_str(format!("{price}").as_str()).unwrap();
                let p = f.format(&p);
                format!("¥{p}")
            }
            PriceCheckService::TcgPlayer => {
                let f = DecimalFormatter::try_new(locale!("en-US").into(), Default::default())
                    .expect("locale should be present");
                let p = Decimal::from_str(format!("{price:.2}").as_str()).unwrap();
                let p = f.format(&p);
                format!("${p} USD")
            }
        };
        format!("{approx_price}{price}")
    }
}
