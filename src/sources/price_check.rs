use std::{collections::HashMap, error::Error, sync::OnceLock};

use dioxus::{
    logger::tracing::{debug, error},
    prelude::*,
};
use hocg_fan_sim_prices_model::{Price, PricesDatabase, ServiceId};
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

#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
pub enum PriceCheckService {
    Yuyutei,
    TcgPlayer,
}

fn price_lookup_key(key: &PriceCacheKey) -> ServiceId {
    match key {
        PriceCacheKey::Yuyutei(url) => ServiceId::from_yuyutei(url.clone()),
        PriceCacheKey::TcgPlayer(product_id) => ServiceId::from_tcgplayer(*product_id),
    }
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
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
                    Yuyutei => i.yuyutei_sell_url.is_some(),
                    TcgPlayer => i.tcgplayer_product_id.is_some(),
                }
            } else {
                false
            }
        })
        .any(|c| {
            c.price_cache(db, prices, service)
                .map(|(cache_time, _)| {
                    // more than an hour
                    Timestamp::now().duration_since(*cache_time) > SignedDuration::from_hours(1)
                })
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
        .filter_map(|(c, service)| {
            Some(match service {
                Yuyutei => PriceCacheKey::Yuyutei(c.yuyutei_sell_url.as_ref()?.to_string()),
                TcgPlayer => PriceCacheKey::TcgPlayer(c.tcgplayer_product_id?),
            })
        })
        .filter_map(|key| {
            let (_timestamp, price) = shared_prices.get(&price_lookup_key(&key))?;
            Some((key, *price))
        })
        .map(|(key, price)| (key, (Timestamp::now(), price)))
        .collect();
    debug!("{:?}", prices);

    Ok(prices)
}

fn tcgplayer_mass_entry_url(
    deck: &DeckOrPile,
    free_basic_cheers: bool,
    db: &CardsDatabase,
) -> String {
    let product_ids = deck
        .all_cards()
        .filter(|c| !c.is_basic_cheer() || !free_basic_cheers)
        .filter_map(|c| {
            Some(format!(
                "{}-{}",
                c.amount,
                c.card_illustration(db)?.tcgplayer_product_id?
            ))
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
                .and_then(|c| c.tcgplayer_product_id)
                .is_none()
        })
    });

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
                    c.price(
                        &db.read(),
                        &prices.read(),
                        *price_service.read(),
                        *FREE_BASIC_CHEERS.read(),
                    )
                    .is_some()
                })
                .sorted_by_key(|c| {
                    std::cmp::Reverse(
                        c.price(
                            &db.read(),
                            &prices.read(),
                            *price_service.read(),
                            *FREE_BASIC_CHEERS.read(),
                        )
                        .expect("it's some"),
                    )
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
                    c.price(
                        &db.read(),
                        &prices.read(),
                        *price_service.read(),
                        *FREE_BASIC_CHEERS.read(),
                    )
                    .is_some()
                })
                .sorted_by_key(|c| {
                    c.price(
                        &db.read(),
                        &prices.read(),
                        *price_service.read(),
                        *FREE_BASIC_CHEERS.read(),
                    )
                    .expect("it's some")
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

    let tcgplayer_mass_entry = move |_| {
        let url =
            tcgplayer_mass_entry_url(&common_deck.read(), *FREE_BASIC_CHEERS.read(), &db.read());
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
                        option { value: "yuyutei", "Yuyutei (JPY)" }
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
    }
}
