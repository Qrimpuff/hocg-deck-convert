use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::CardsDatabase;
use itertools::Itertools;
use serde::Serialize;

use crate::{
    CARDS_PRICES, CardType, EventType, FREE_BASIC_CHEERS, PRICE_SERVICE,
    sources::{
        CommonCard, DeckLike, DeckOrPile,
        price_check::{
            DeckHistoryCard, PriceCacheKey, PriceCheckService, PriceHistorySeries, current_prices,
            fetch_deck_price_history, fetch_price_history,
        },
    },
    track_event,
    tracker::TrackEvent,
};

#[derive(Serialize)]
struct PriceHistoryUse<'a> {
    subject: &'a str,
    service: &'a str,
}

impl TrackEvent for PriceHistoryUse<'_> {}

/// Price history chart for the selected card illustration and price service.
#[component]
pub fn PriceHistory(
    chart_id: String,
    card: Signal<CommonCard>,
    db: Signal<CardsDatabase>,
) -> Element {
    let is_cheer = use_memo(move || {
        let db = db.read();
        card.read().card_type(&db) == Some(CardType::Cheer)
    });

    let keys = use_memo(move || {
        let db = db.read();
        let Some(illust) = card.read().card_illustration(&db) else {
            return vec![];
        };

        match *PRICE_SERVICE.read() {
            PriceCheckService::Yuyutei => illust
                .yuyutei_sell_paths
                .iter()
                .flatten()
                .cloned()
                .map(PriceCacheKey::Yuyutei)
                .collect_vec(),
            PriceCheckService::TcgPlayer => illust
                .tcgplayer_product_ids
                .iter()
                .flatten()
                .copied()
                .map(PriceCacheKey::TcgPlayer)
                .collect_vec(),
        }
    });

    let history = use_resource(move || {
        let keys = keys.read().clone();
        let card_number = card.read().card_number.to_string();
        let is_cheer = *is_cheer.read();
        let current = current_prices(&keys, &CARDS_PRICES.read());
        async move {
            fetch_price_history(&card_number, is_cheer, &keys, &current)
                .await
                .unwrap_or_default()
        }
    });

    price_chart(chart_id, "Price history", "card", history)
}

/// Total price history of the whole deck or pile.
#[component]
pub fn DeckPriceHistory(
    chart_id: String,
    common_deck: Signal<DeckOrPile>,
    db: Signal<CardsDatabase>,
) -> Element {
    let history = use_resource(move || {
        let service = *PRICE_SERVICE.read();
        let free_basic_cheers = *FREE_BASIC_CHEERS.read();
        let cards = {
            let db = db.read();
            common_deck
                .read()
                .all_cards()
                .filter(|card| !(free_basic_cheers && card.is_basic_cheer()))
                .map(|card| DeckHistoryCard {
                    card_number: card.card_number.clone(),
                    is_cheer: card.card_type(&db) == Some(CardType::Cheer),
                    keys: card.price_keys(&db, service),
                    amount: card.amount as f64,
                })
                .collect_vec()
        };
        let current = current_prices(
            cards.iter().flat_map(|card| card.keys.iter()),
            &CARDS_PRICES.read(),
        );
        async move {
            let data = fetch_deck_price_history(&cards, &current).await;
            if data.is_empty() {
                return vec![];
            }
            vec![PriceHistorySeries {
                name: "Total".into(),
                currency: service.currency(),
                data,
            }]
        }
    });

    let (title, subject) = if matches!(*common_deck.read(), DeckOrPile::Pile(_)) {
        ("Pile price history", "pile")
    } else {
        ("Deck price history", "deck")
    };
    price_chart(chart_id, title, subject, history)
}

/// Chart with loading and empty states. The chart is drawn once data is available and disposed on unmount.
fn price_chart(
    chart_id: String,
    title: &str,
    subject: &'static str,
    history: Resource<Vec<PriceHistorySeries>>,
) -> Element {
    let chart_id_effect = chart_id.clone();
    use_effect(move || {
        let history = history.read();
        if let Some(series) = history.as_ref().filter(|s| !s.is_empty()) {
            let json = serde_json::to_string(series).unwrap_or_else(|_| "[]".into());
            document().eval(format!(
                "window.renderPriceHistory('{chart_id_effect}', {json});"
            ));
            let chart_id_watch = chart_id_effect.clone();
            spawn(async move {
                let mut usage = document().eval(format!(
                    "window.watchPriceHistoryUse('{chart_id_watch}', dioxus);"
                ));
                if usage.recv::<bool>().await.is_ok() {
                    let service = PRICE_SERVICE.peek().name();
                    track_event(
                        EventType::PriceHistory,
                        PriceHistoryUse { subject, service },
                    );
                }
            });
        }
    });

    let loading = history.read().is_none();
    let chart_id_drop = chart_id.clone();
    use_drop(move || {
        document().eval(format!("window.disposePriceHistory('{chart_id_drop}');"));
    });
    let has_data = history.read().as_ref().is_some_and(|s| !s.is_empty());
    let service = PRICE_SERVICE.read().name();

    rsx! {
        div { class: "block", style: "margin-top: 1.5rem;",
            p { class: "has-text-weight-semibold", "{title} ({service})" }
            if loading {
                p { class: "is-size-7", "Loading..." }
            }
            if !loading && !has_data {
                p { class: "is-size-7", "No price history available." }
            }
            if has_data {
                div { id: "{chart_id}", style: "width: 100%; height: 220px;" }
            }
        }
    }
}
