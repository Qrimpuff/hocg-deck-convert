use std::{collections::HashMap, error::Error, sync::OnceLock};

use dioxus::{
    logger::tracing::{debug, error},
    prelude::*,
};
use hocg_fan_sim_prices_model::{Price, PricesDatabase, PricesHistoryDatabase, ServiceId};
use itertools::Itertools;
use jiff::{SignedDuration, Timestamp};
use reqwest::{Client, ClientBuilder};
use serde::Serialize;

use super::CardsDatabase;
use crate::{
    CardLanguage, EventType, FREE_BASIC_CHEERS, PREVIEW_CARD_LANG,
    sources::{
        DeckLike, DeckOrPile,
        price_check::PriceCheckService::{TcgPlayer, Yuyutei},
    },
    track_event,
    tracker::{TrackEvent, track_external_url},
};

const HOCG_FAN_SIM_PRICES_URL: &str =
    "https://qrimpuff.github.io/hocg-fan-sim-prices/hocg_prices.json";

pub type PriceCache = HashMap<PriceCacheKey, (Timestamp, Price)>;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum PriceCacheKey {
    Yuyutei(String),
    TcgPlayer(u32),
}

impl PriceCacheKey {
    pub fn service_id(&self) -> ServiceId {
        match self {
            PriceCacheKey::Yuyutei(sell_path) => ServiceId::from_yuyutei(sell_path.clone()),
            PriceCacheKey::TcgPlayer(product_id) => ServiceId::from_tcgplayer(*product_id),
        }
    }

    pub fn service(&self) -> PriceCheckService {
        match self {
            PriceCacheKey::Yuyutei(_) => PriceCheckService::Yuyutei,
            PriceCacheKey::TcgPlayer(_) => PriceCheckService::TcgPlayer,
        }
    }
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
pub enum PriceCheckService {
    Yuyutei,
    TcgPlayer,
}

impl PriceCheckService {
    pub fn name(self) -> &'static str {
        match self {
            PriceCheckService::Yuyutei => "Yuyu-tei",
            PriceCheckService::TcgPlayer => "TCGplayer",
        }
    }

    pub fn currency(self) -> &'static str {
        match self {
            PriceCheckService::Yuyutei => "¥",
            PriceCheckService::TcgPlayer => "$",
        }
    }
}

/// The current prices of the given keys, for the keys that have a price.
pub fn current_prices<'a>(
    keys: impl IntoIterator<Item = &'a PriceCacheKey>,
    prices: &PriceCache,
) -> PriceCache {
    keys.into_iter()
        .filter_map(|key| prices.get(key).map(|price| (key.clone(), *price)))
        .collect()
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

/// Whether a cache entry is less than an hour old.
fn is_fresh(cache_time: Timestamp) -> bool {
    Timestamp::now().duration_since(cache_time) <= SignedDuration::from_hours(1)
}

async fn price_check(
    deck: &DeckOrPile,
    db: &CardsDatabase,
    prices: &PriceCache,
    service: PriceCheckService,
) -> Result<PriceCache, Box<dyn Error>> {
    debug!("price check");

    // read price from cache
    let need_prices = deck
        .all_cards()
        // check price for all versions
        .flat_map(|c| c.alt_cards(db).into_iter())
        .filter(|c| {
            if let Some(i) = c.card_illustration(db) {
                match service {
                    Yuyutei => i.yuyutei_sell_paths.is_some(),
                    TcgPlayer => i.tcgplayer_product_ids.is_some(),
                }
            } else {
                false
            }
        })
        .any(|c| {
            c.price_cache(db, prices, service)
                .map(|(_, (cache_time, _))| !is_fresh(*cache_time))
                .unwrap_or(true)
        });
    if !need_prices {
        return Ok(PriceCache::new());
    }

    // otherwise, fetch the prices
    let resp = http_client()
        .get(HOCG_FAN_SIM_PRICES_URL)
        .send()
        .await
        .map_err(|err| {
            error!("Failed to fetch prices from hocg-fan-sim-prices: {err}");
            "service unavailable"
        })?;

    let content = resp.text().await.unwrap();
    debug!("loaded shared prices db ({} bytes)", content.len());

    let shared_prices: PricesDatabase = serde_json::from_str(&content).map_err(|_| content)?;

    // it contains all the prices
    let prices: PriceCache = db
        .values()
        .flat_map(|c| &c.illustrations)
        .cartesian_product([Yuyutei, TcgPlayer])
        .flat_map(|(c, service)| match service {
            Yuyutei => c
                .yuyutei_sell_paths
                .iter()
                .flatten()
                .map(|path| PriceCacheKey::Yuyutei(path.clone()))
                .collect_vec(),
            TcgPlayer => c
                .tcgplayer_product_ids
                .iter()
                .flatten()
                .map(|id| PriceCacheKey::TcgPlayer(*id))
                .collect_vec(),
        })
        .filter_map(|key| {
            let (_timestamp, price) = shared_prices.get(&key.service_id())?;
            Some((key, *price))
        })
        .map(|(key, price)| (key, (Timestamp::now(), price)))
        .collect();
    debug!("{:?}", prices);

    Ok(prices)
}

const HOCG_FAN_SIM_PRICES_BASE_URL: &str = "https://qrimpuff.github.io/hocg-fan-sim-prices";

/// A price history series for the chart. `data` is `(timestamp in milliseconds, price, approximate)`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PriceHistorySeries {
    pub name: String,
    pub currency: &'static str,
    pub data: Vec<(i64, f64, bool)>,
}

/// The history file that contains the card, split by card type and set.
fn history_file_url(card_number: &str, is_cheer: bool) -> String {
    let set = card_number.split('-').next().unwrap_or_default();
    if is_cheer {
        format!("{HOCG_FAN_SIM_PRICES_BASE_URL}/hY/{set}_history.json")
    } else {
        format!("{HOCG_FAN_SIM_PRICES_BASE_URL}/{set}/{card_number}_history.json")
    }
}

fn price_value(price: &Price) -> f64 {
    match price {
        Price::Zero => 0.0,
        Price::Yen(yen) => *yen as f64,
        // dollars are stored in cents
        Price::Dollar(cents) => *cents as f64 / 100.0,
    }
}

type PriceHistoryCache = HashMap<String, (Timestamp, PricesHistoryDatabase)>;

// history files by URL
static PRICE_HISTORY_CACHE: GlobalSignal<PriceHistoryCache> = Signal::global(Default::default);

/// Downloads the history file into the cache if it is missing or older than an hour.
async fn ensure_price_history_file(url: &str) -> Result<(), Box<dyn Error>> {
    let fresh = PRICE_HISTORY_CACHE
        .peek()
        .get(url)
        .is_some_and(|(cache_time, _)| is_fresh(*cache_time));
    if fresh {
        return Ok(());
    }

    debug!("loading price history: {url}");
    let resp = http_client().get(url).send().await.map_err(|err| {
        error!("Failed to fetch price history from hocg-fan-sim-prices: {err}");
        "service unavailable"
    })?;

    let history: PricesHistoryDatabase = if resp.status().is_success() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(|_| "invalid price history")?
    } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
        // no history file for this card, cache the miss
        PricesHistoryDatabase::new()
    } else {
        // not cached, so the next request retries
        return Err(format!("price history unavailable: {}", resp.status()).into());
    };

    PRICE_HISTORY_CACHE
        .write()
        .insert(url.to_string(), (Timestamp::now(), history));
    Ok(())
}

/// Fetches the price history of a card for the given price keys (one series per key).
pub async fn fetch_price_history(
    card_number: &str,
    is_cheer: bool,
    keys: &[PriceCacheKey],
    current: &PriceCache,
) -> Result<Vec<PriceHistorySeries>, Box<dyn Error>> {
    if keys.is_empty() {
        return Ok(vec![]);
    }

    let url = history_file_url(card_number, is_cheer);
    ensure_price_history_file(&url).await?;

    let cache = PRICE_HISTORY_CACHE.peek();
    let Some((_, history)) = cache.get(&url) else {
        return Ok(vec![]);
    };

    let now = Timestamp::now();
    let series: Vec<PriceHistorySeries> = keys
        .iter()
        .enumerate()
        .filter_map(|(i, key)| {
            let points = history.get(&key.service_id())?;
            let service = key.service();
            let name = if keys.len() > 1 {
                format!("{} {}", service.name(), i + 1)
            } else {
                service.name().to_string()
            };
            let mut points = points.clone();
            if let Some((_, price)) = current.get(key) {
                let today = now.as_second().div_euclid(86_400);
                if points
                    .last()
                    .is_some_and(|(ts, _)| ts.as_second().div_euclid(86_400) == today)
                {
                    if let Some(last) = points.last_mut() {
                        last.1 = *price;
                    }
                } else {
                    points.push((now, *price));
                }
            }
            Some(PriceHistorySeries {
                name,
                currency: service.currency(),
                data: points
                    .iter()
                    .map(|(ts, price)| (ts.as_millisecond(), price_value(price), false))
                    .collect(),
            })
        })
        .filter(|s| !s.data.is_empty())
        .collect();

    Ok(series)
}

/// A card in a deck or pile, with what is needed to fetch its price history.
pub struct DeckHistoryCard {
    pub card_number: String,
    pub is_cheer: bool,
    pub keys: Vec<PriceCacheKey>,
    pub amount: f64,
}

/// Total price history; each point is flagged when some card has no price at that time.
pub async fn fetch_deck_price_history(
    cards: &[DeckHistoryCard],
    current: &PriceCache,
) -> Vec<(i64, f64, bool)> {
    let mut card_series = Vec::with_capacity(cards.len());
    for card in cards {
        let series = fetch_price_history(&card.card_number, card.is_cheer, &card.keys, current)
            .await
            .unwrap_or_default();
        let data = series
            .into_iter()
            .map(|s| {
                s.data
                    .into_iter()
                    .map(|(ts, price, _)| (ts, price))
                    .collect_vec()
            })
            .collect_vec();
        card_series.push((card.amount, data));
    }
    combine_price_histories(card_series)
}

/// One card's price series per service, with the card amount.
type CardSeries = (f64, Vec<Vec<(i64, f64)>>);

/// Sums each card's best-price step series; a time is approximate when some card has no price yet.
fn combine_price_histories(cards: Vec<CardSeries>) -> Vec<(i64, f64, bool)> {
    let card_totals: Vec<(f64, Vec<(i64, f64)>)> = cards
        .into_iter()
        .map(|(amount, mut series)| {
            series.iter_mut().for_each(|s| s.sort_by_key(|(ts, _)| *ts));
            let times = series
                .iter()
                .flatten()
                .map(|(ts, _)| *ts)
                .sorted()
                .dedup()
                .collect_vec();
            let best = times
                .into_iter()
                .filter_map(|ts| {
                    series
                        .iter()
                        .filter_map(|s| step_value(s, ts))
                        .min_by(f64::total_cmp)
                        .map(|price| (ts, price))
                })
                .collect_vec();
            (amount, best)
        })
        .collect();

    let times = card_totals
        .iter()
        .flat_map(|(_, s)| s.iter().map(|(ts, _)| *ts))
        .sorted()
        .dedup()
        .collect_vec();
    times
        .into_iter()
        .map(|ts| {
            let mut total = 0.0;
            let mut approximate = false;
            for (amount, s) in &card_totals {
                match step_value(s, ts) {
                    Some(price) => total += price * amount,
                    None => approximate = true,
                }
            }
            (ts, total, approximate)
        })
        .collect()
}

/// The last value at or before `ts` in a sorted step series.
fn step_value(series: &[(i64, f64)], ts: i64) -> Option<f64> {
    let count = series.partition_point(|(t, _)| *t <= ts);
    count.checked_sub(1).map(|i| series[i].1)
}

fn tcgplayer_mass_entry_url(
    deck: &DeckOrPile,
    free_basic_cheers: bool,
    db: &CardsDatabase,
    prices: &PriceCache,
) -> String {
    let product_ids = deck
        .all_cards()
        .filter(|c| !c.is_basic_cheer() || !free_basic_cheers)
        .filter_map(|c| {
            // find the product id with the best price
            let product_id = c
                .price(db, prices, PriceCheckService::TcgPlayer, free_basic_cheers)
                .and_then(|key| match key.0? {
                    PriceCacheKey::TcgPlayer(id) => Some(id),
                    _ => None,
                })
                .or_else(|| {
                    c.card_illustration(db)
                        .and_then(|illustration| illustration.tcgplayer_product_ids.as_ref())
                        .and_then(|ids| ids.first())
                        .copied()
                })?;
            Some(format!("{}-{}", c.amount, product_id))
        })
        .collect::<Vec<_>>()
        .join("||");
    format!(
        "https://www.tcgplayer.com/massentry?c={product_ids}&productline=hololive%20OFFICIAL%20CARD%20GAME"
    )
}

#[component]
pub fn Export(
    mut common_deck: Signal<DeckOrPile>,
    db: Signal<CardsDatabase>,
    prices: Signal<PriceCache>,
    mut price_service: Signal<PriceCheckService>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        price_check_service: Option<PriceCheckService>,
        #[serde(skip_serializing_if = "Option::is_none")]
        price_check_convert: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }
    impl TrackEvent for EventData {}

    let mut deck_error = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let has_prices = use_memo(move || {
        prices.read().keys().any(|key| match *price_service.read() {
            Yuyutei => matches!(key, PriceCacheKey::Yuyutei(_)),
            TcgPlayer => matches!(key, PriceCacheKey::TcgPlayer(_)),
        })
    });

    let has_missing_tcgplayer_ids = use_memo(move || {
        common_deck.read().all_cards().any(|c| {
            c.card_illustration(&db.read())
                .and_then(|c| c.tcgplayer_product_ids.as_ref())
                .is_none()
        })
    });

    let price_check = move || async move {
        let common_deck = common_deck.read();

        *loading.write() = true;
        *deck_error.write() = String::new();

        let price_check = price_check(
            &common_deck,
            &db.read(),
            &prices.read(),
            *price_service.read(),
        )
        .await;
        match price_check {
            Ok(price_check) => {
                prices.write().extend(price_check);
                *show_price.write() = true;
                track_event(
                    EventType::Export("Price check".into()),
                    EventData {
                        format: "Price check",
                        price_check_service: Some(*price_service.read()),
                        price_check_convert: None,
                        error: None,
                    },
                );
            }
            Err(e) => {
                *deck_error.write() = e.to_string();
                track_event(
                    EventType::Export("Price check".into()),
                    EventData {
                        format: "Price check",
                        price_check_service: Some(*price_service.read()),
                        price_check_convert: None,
                        error: Some(e.to_string()),
                    },
                );
            }
        }

        *loading.write() = false;
    };

    use_effect(move || {
        let _ = price_service.read();
        if !common_deck.peek().is_empty() {
            spawn(price_check());
        }
    });

    let increase_price = move |_| {
        let mut common_deck = common_deck.write();

        *loading.write() = true;
        *deck_error.write() = String::new();

        let mut deck = common_deck.clone();
        for card in deck.all_cards_mut() {
            if let Some((_, alt_card)) = card
                .alt_cards(&db.read())
                .into_iter()
                .filter_map(|c| {
                    Some((
                        c.price(
                            &db.read(),
                            &prices.read(),
                            *price_service.read(),
                            *FREE_BASIC_CHEERS.read(),
                        )?
                        .1,
                        c,
                    ))
                })
                .sorted_by_key(|(price, _)| std::cmp::Reverse(*price)) // this is the highest price
                .next()
            {
                card.card_number = alt_card.card_number; // it could be a cheer card
                card.illustration_idx = alt_card.illustration_idx;
            }
        }
        deck.merge();
        *common_deck = deck;

        track_event(
            EventType::Export("Price check".into()),
            EventData {
                format: "Price check",
                price_check_service: None,
                price_check_convert: Some("highest price".into()),
                error: None,
            },
        );

        *loading.write() = false;
    };

    let decrease_price = move |_| {
        let mut common_deck = common_deck.write();

        *loading.write() = true;
        *deck_error.write() = String::new();

        let mut deck = common_deck.clone();
        for card in deck.all_cards_mut() {
            if let Some((_, alt_card)) = card
                .alt_cards(&db.read())
                .into_iter()
                .filter_map(|c| {
                    Some((
                        c.price(
                            &db.read(),
                            &prices.read(),
                            *price_service.read(),
                            *FREE_BASIC_CHEERS.read(),
                        )?
                        .1,
                        c,
                    ))
                })
                .sorted_by_key(|(price, _)| *price) // this is the lowest price
                .next()
            {
                card.card_number = alt_card.card_number; // it could be a cheer card
                card.illustration_idx = alt_card.illustration_idx;
            }
        }
        deck.merge();
        *common_deck = deck;

        track_event(
            EventType::Export("Price check".into()),
            EventData {
                format: "Price check",
                price_check_service: None,
                price_check_convert: Some("lowest price".into()),
                error: None,
            },
        );

        *loading.write() = false;
    };

    let tcgplayer_mass_entry = move |_| {
        let url = tcgplayer_mass_entry_url(
            &common_deck.read(),
            *FREE_BASIC_CHEERS.read(),
            &db.read(),
            &prices.read(),
        );
        web_sys::window().unwrap().open_with_url(&url).unwrap();

        track_external_url("TCGplayer - Mass Entry");
    };

    rsx! {

        div { class: "field",
            label { "for": "service", class: "label", "Service" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "service",
                        oninput: move |ev| {
                            *show_price.write() = true;
                            *price_service.write() = match ev.value().as_str() {
                                "yuyutei" => Yuyutei,
                                "tcgplayer" => TcgPlayer,
                                _ => unreachable!(),
                            };
                            *PREVIEW_CARD_LANG.write() = match ev.value().as_str() {
                                "yuyutei" => CardLanguage::Japanese,
                                "tcgplayer" => CardLanguage::English,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "yuyutei", "Yuyu-tei (JPY)" }
                        option { value: "tcgplayer", "TCGplayer (USD)" }
                    }
                }
            }
        }

        div { class: "field",
            div { class: "control",
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: *FREE_BASIC_CHEERS.read(),
                        onclick: move |_| {
                            *FREE_BASIC_CHEERS.write() ^= true;
                        },
                    }
                    " Free basic cheers"
                }
            }
        }

        div { class: "field",
            div { class: "control",
                button {
                    r#type: "button",
                    class: "button",
                    class: if *loading.read() { "is-loading" },
                    disabled: common_deck.read().is_empty() || *loading.read(),
                    onclick: move |_| price_check(),
                    span { class: "icon",
                        i { class: "fa-solid fa-tag" }
                    }
                    span { "Refresh price" }
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }

        br {}

        div { class: "field",
            div { class: "control",
                button {
                    r#type: "button",
                    class: "button",
                    disabled: common_deck.read().is_empty() || *loading.read() || !*has_prices.read(),
                    onclick: increase_price,
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-up" }
                    }
                    span { "Convert to highest price" }
                }
            }
        }

        div { class: "field",
            div { class: "control",
                button {
                    r#type: "button",
                    class: "button",
                    disabled: common_deck.read().is_empty() || *loading.read() || !*has_prices.read(),
                    onclick: decrease_price,
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-down" }
                    }
                    span { "Convert to lowest price" }
                }
            }
        }

        if *price_service.read() == TcgPlayer {
            br {}

            if *has_missing_tcgplayer_ids.read() {
                div { class: "field",
                    p { class: "notification is-warning",
                        "Some cards are missing from TCGplayer, so they will not be included in the mass entry."
                    }
                }
            }

            div { class: "field",
                div { class: "control",
                    button {
                        class: "button",
                        disabled: common_deck.read().is_empty() || *loading.read(),
                        onclick: tcgplayer_mass_entry,
                        span { class: "icon",
                            i { class: "fa-solid fa-external-link" }
                        }
                        span { "TCGplayer Mass Entry" }
                    }
                }
            }
        }
        if !common_deck.read().is_empty() {
            br {}
            crate::components::price_history::DeckPriceHistory {
                chart_id: "deck-price-history".to_string(),
                common_deck,
                db,
            }
        }
    }
}
