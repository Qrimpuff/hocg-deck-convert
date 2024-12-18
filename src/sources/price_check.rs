use std::{collections::HashMap, error::Error, sync::OnceLock};

use dioxus::{
    logger::tracing::{debug, info},
    prelude::*,
};
use itertools::Itertools;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use web_time::{Duration, Instant};

use super::{CardsInfo, CommonDeck};
use crate::{track_event, CardLanguage, EventType, HOCG_DECK_CONVERT_API};

pub type PriceCache = HashMap<String, (Instant, u32)>;

#[derive(Clone, Copy, Serialize)]
enum PriceCheckService {
    Yuyutei,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PriceCheckRequest {
    urls: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PriceCheckResult {
    url: String,
    card_number: String,
    price_yen: u32,
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

async fn price_check(
    deck: &CommonDeck,
    info: &CardsInfo,
    prices: &PriceCache,
    service: PriceCheckService,
) -> Result<PriceCache, Box<dyn Error>> {
    info!("price check");

    // read price from cache
    let urls: Vec<_> = deck
        .all_cards()
        // check price for all versions
        .flat_map(|c| c.alt_cards(info).into_iter())
        .filter(|c| {
            c.price_cache(info, prices)
                .map(|(cache_time, _)| {
                    // more than an hour
                    Instant::now().duration_since(*cache_time) > Duration::from_secs(60 * 60)
                })
                .unwrap_or(true)
        })
        .filter_map(|c| c.card_info(info))
        .filter_map(|c| match service {
            PriceCheckService::Yuyutei => c.yuyutei_sell_url.clone(),
        })
        .unique()
        .collect();
    if urls.is_empty() {
        return Ok(PriceCache::new());
    }

    let req = PriceCheckRequest { urls };

    let resp = http_client()
        .post(format!("{HOCG_DECK_CONVERT_API}/price-check"))
        .json(&req)
        .send()
        .await
        .unwrap();

    let content = resp.text().await.unwrap();
    debug!("{:?}", content);

    let res: Vec<PriceCheckResult> = serde_json::from_str(&content).map_err(|_| content)?;
    let lookup_prices: HashMap<_, _> = res
        .into_iter()
        .map(|r| (r.url, (r.card_number, r.price_yen)))
        .collect();
    debug!("{:?}", lookup_prices);

    // update the price
    let mut prices = PriceCache::new();
    for card in deck.all_cards() {
        for card in card
            .alt_cards(info)
            .into_iter()
            .filter_map(|c| c.card_info(info))
        {
            if let Some(url) = match service {
                PriceCheckService::Yuyutei => &card.yuyutei_sell_url,
            } {
                lookup_prices
                    .get(url)
                    .map(|p| prices.insert(url.into(), (Instant::now(), p.1)));
            }
        }
    }

    Ok(prices)
}

#[component]
pub fn Export(
    mut common_deck: Signal<Option<CommonDeck>>,
    info: Signal<CardsInfo>,
    prices: Signal<PriceCache>,
    card_lang: Signal<CardLanguage>,
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
    let mut service = use_signal(|| PriceCheckService::Yuyutei);
    let mut loading = use_signal(|| false);

    let price_check = move |_| async move {
        let common_deck = common_deck.read();
        let Some(common_deck) = common_deck.as_ref() else {
            return;
        };

        *loading.write() = true;
        *deck_error.write() = String::new();

        let price_check =
            price_check(common_deck, &info.read(), &prices.read(), *service.read()).await;
        match price_check {
            Ok(price_check) => {
                prices.write().extend(price_check);
                *show_price.write() = true;
                track_event(
                    EventType::Export("Price check".into()),
                    EventData {
                        format: "Price check",
                        price_check_service: Some(*service.read()),
                        price_check_convert: None,
                        error: None,
                    },
                )
                .await;
            }
            Err(e) => {
                *deck_error.write() = e.to_string();
                track_event(
                    EventType::Export("Price check".into()),
                    EventData {
                        format: "Price check",
                        price_check_service: Some(*service.read()),
                        price_check_convert: None,
                        error: Some(e.to_string()),
                    },
                )
                .await;
            }
        }

        *loading.write() = false;
    };

    let increase_price = move |_| async move {
        let mut common_deck = common_deck.write();
        let Some(common_deck) = common_deck.as_mut() else {
            return;
        };

        *loading.write() = true;
        *deck_error.write() = String::new();

        let mut deck = common_deck.clone();
        for card in deck.all_cards_mut().filter(|c| c.manage_id.is_some()) {
            if let Some(manage_id) = card
                .alt_cards(&info.read())
                .into_iter()
                .filter(|c| c.price(&info.read(), &prices.read()).is_some())
                .sorted_by_key(|c| {
                    u32::MAX - c.price(&info.read(), &prices.read()).expect("it's some")
                }) // this is the highest price
                .map(|c| c.manage_id)
                .next()
            {
                card.manage_id = manage_id;
            }
        }
        *common_deck = deck.merge();

        track_event(
            EventType::Export("Price check".into()),
            EventData {
                format: "Price check",
                price_check_service: None,
                price_check_convert: Some("highest price".into()),
                error: None,
            },
        )
        .await;

        *loading.write() = false;
    };

    let decrease_price = move |_| async move {
        let mut common_deck = common_deck.write();
        let Some(common_deck) = common_deck.as_mut() else {
            return;
        };

        *loading.write() = true;
        *deck_error.write() = String::new();

        let mut deck = common_deck.clone();
        for card in deck.all_cards_mut().filter(|c| c.manage_id.is_some()) {
            if let Some(manage_id) = card
                .alt_cards(&info.read())
                .into_iter()
                .filter(|c| c.price(&info.read(), &prices.read()).is_some())
                .sorted_by_key(|c| c.price(&info.read(), &prices.read()).expect("it's some")) // this is the lowest price
                .map(|c| c.manage_id)
                .next()
            {
                card.manage_id = manage_id;
            }
        }
        *common_deck = deck.merge();

        track_event(
            EventType::Export("Price check".into()),
            EventData {
                format: "Price check",
                price_check_service: None,
                price_check_convert: Some("lowest price".into()),
                error: None,
            },
        )
        .await;

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
                            *service.write() = match ev.value().as_str() {
                                "yuyutei" => PriceCheckService::Yuyutei,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "yuyutei", "Yuyutei" }
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
                    disabled: common_deck.read().is_none() || *loading.read(),
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
                    disabled: common_deck.read().is_none() || *loading.read() || !*show_price.read(),
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
                    disabled: common_deck.read().is_none() || *loading.read() || !*show_price.read(),
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
