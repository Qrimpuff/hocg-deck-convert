use std::{collections::HashMap, error::Error, sync::OnceLock};

use dioxus::{
    logger::tracing::{debug, error},
    prelude::*,
};
use itertools::Itertools;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use web_time::{Duration, Instant};

use super::{CardsDatabase, CommonDeck};
use crate::{CardLanguage, EventType, HOCG_DECK_CONVERT_API, PREVIEW_CARD_LANG, track_event};

pub type PriceCache = HashMap<PriceCacheKey, (Instant, f64)>;
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum PriceCacheKey {
    Yuyutei(String),
    TcgPlayer(u32),
}

#[derive(Clone, Copy, Serialize)]
pub enum PriceCheckService {
    Yuyutei,
    TcgPlayer,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct YuyuteiPriceCheckRequest {
    urls: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct YuyuteiPriceCheckResult {
    url: String,
    card_number: String,
    price_yen: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TcgPlayerPriceCheckRequest {
    product_ids: Vec<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct TcgPlayerPriceCheckResult {
    product_id: u32,
    price_usd: f64,
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

async fn price_check(
    deck: &CommonDeck,
    db: &CardsDatabase,
    prices: &PriceCache,
    service: PriceCheckService,
) -> Result<PriceCache, Box<dyn Error>> {
    debug!("price check");

    // read price from cache
    let keys: Vec<_> = deck
        .all_cards()
        // check price for all versions
        .flat_map(|c| c.alt_cards(db).into_iter())
        .filter(|c| {
            c.price_cache(db, prices, service)
                .map(|(cache_time, _)| {
                    // more than an hour
                    Instant::now().duration_since(*cache_time) > Duration::from_secs(60 * 60)
                })
                .unwrap_or(true)
        })
        .filter_map(|c| c.card_illustration(db))
        .filter_map(|c| {
            Some(match service {
                PriceCheckService::Yuyutei => {
                    PriceCacheKey::Yuyutei(c.yuyutei_sell_url.as_ref()?.to_string())
                }
                PriceCheckService::TcgPlayer => PriceCacheKey::TcgPlayer(c.tcgplayer_product_id?),
            })
        })
        .unique()
        .collect();
    if keys.is_empty() {
        return Ok(PriceCache::new());
    }

    let lookup_prices: HashMap<_, _> = {
        match service {
            PriceCheckService::Yuyutei => {
                let req = YuyuteiPriceCheckRequest {
                    urls: keys
                        .into_iter()
                        .map(|k| match k {
                            PriceCacheKey::Yuyutei(url) => url,
                            _ => unreachable!(),
                        })
                        .collect(),
                };

                let resp = http_client()
                    .post(format!("{HOCG_DECK_CONVERT_API}/price-check-yuyutei"))
                    .json(&req)
                    .send()
                    .await
                    .map_err(|err| {
                        error!("Failed to fetch prices from Yuyutei: {err}");
                        "service unavailable"
                    })?;

                let content = resp.text().await.unwrap();
                debug!("{:?}", content);

                let res: Vec<YuyuteiPriceCheckResult> =
                    serde_json::from_str(&content).map_err(|_| content)?;
                res.into_iter()
                    .map(|r| (PriceCacheKey::Yuyutei(r.url), r.price_yen as f64))
                    .collect()
            }
            PriceCheckService::TcgPlayer => {
                let req = TcgPlayerPriceCheckRequest {
                    product_ids: keys
                        .into_iter()
                        .map(|k| match k {
                            PriceCacheKey::TcgPlayer(id) => id,
                            _ => unreachable!(),
                        })
                        .collect(),
                };

                let resp = http_client()
                    .post(format!("{HOCG_DECK_CONVERT_API}/price-check-tcgplayer"))
                    .json(&req)
                    .send()
                    .await
                    .map_err(|err| {
                        error!("Failed to fetch prices from TCGPlayer: {err}");
                        "service unavailable"
                    })?;

                let content = resp.text().await.unwrap();
                debug!("{:?}", content);

                let res: Vec<TcgPlayerPriceCheckResult> =
                    serde_json::from_str(&content).map_err(|_| content)?;
                res.into_iter()
                    .map(|r| (PriceCacheKey::TcgPlayer(r.product_id), r.price_usd)) // convert to cents
                    .collect()
            }
        }
    };
    debug!("{:?}", lookup_prices);

    // update the price
    let mut prices = PriceCache::new();
    for card in deck.all_cards() {
        for key in card
            .alt_cards(db)
            .into_iter()
            .filter_map(|c| c.card_illustration(db))
            .filter_map(|c| {
                Some(match service {
                    PriceCheckService::Yuyutei => {
                        PriceCacheKey::Yuyutei(c.yuyutei_sell_url.as_ref()?.to_string())
                    }
                    PriceCheckService::TcgPlayer => {
                        PriceCacheKey::TcgPlayer(c.tcgplayer_product_id?)
                    }
                })
            })
        {
            lookup_prices
                .get(&key)
                .map(|p| prices.insert(key, (Instant::now(), *p)));
        }
    }

    Ok(prices)
}

#[component]
pub fn Export(
    mut common_deck: Signal<CommonDeck>,
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

    let mut deck_error = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let price_check = move |_| async move {
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

    let increase_price = move |_| {
        let mut common_deck = common_deck.write();

        *loading.write() = true;
        *deck_error.write() = String::new();

        let mut deck = common_deck.clone();
        for card in deck.all_cards_mut() {
            if let Some(alt_card) = card
                .alt_cards(&db.read())
                .into_iter()
                .filter(|c| {
                    c.price(&db.read(), &prices.read(), *price_service.read())
                        .is_some()
                })
                .sorted_by_key(|c| {
                    u32::MAX
                        - (c.price(&db.read(), &prices.read(), *price_service.read())
                            .expect("it's some")
                            * 100.0) as u32 // convert to cents
                }) // this is the highest price
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
            if let Some(alt_card) = card
                .alt_cards(&db.read())
                .into_iter()
                .filter(|c| {
                    c.price(&db.read(), &prices.read(), *price_service.read())
                        .is_some()
                })
                .sorted_by_key(|c| {
                    (c.price(&db.read(), &prices.read(), *price_service.read())
                        .expect("it's some")
                        * 100.0) as u32 // convert to cents
                }) // this is the lowest price
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

    rsx! {

        div { class: "field",
            label { "for": "service", class: "label", "Service" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "service",
                        oninput: move |ev| {
                            *show_price.write() = false;
                            *price_service.write() = match ev.value().as_str() {
                                "yuyutei" => PriceCheckService::Yuyutei,
                                "tcgplayer" => PriceCheckService::TcgPlayer,
                                _ => unreachable!(),
                            };
                            *PREVIEW_CARD_LANG.write() = match ev.value().as_str() {
                                "yuyutei" => CardLanguage::Japanese,
                                "tcgplayer" => CardLanguage::English,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "yuyutei", "Yuyutei (JPY)" }
                        option { value: "tcgplayer", "TCGPlayer (USD)" }
                    }
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
                    onclick: price_check,
                    span { class: "icon",
                        i { class: "fa-solid fa-tag" }
                    }
                    span { "Check price" }
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
                    disabled: common_deck.read().is_empty() || *loading.read() || !*show_price.read(),
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
                    disabled: common_deck.read().is_empty() || *loading.read() || !*show_price.read(),
                    onclick: decrease_price,
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-down" }
                    }
                    span { "Convert to lowest price" }
                }
            }
        }
    }
}
