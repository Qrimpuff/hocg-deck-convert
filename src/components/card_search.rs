use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::{CardEntry, CardsInfo};
use serde::Serialize;
use web_time::{Duration, Instant};

use crate::{
    CardLanguage, CardType,
    components::card::Card,
    sources::{CommonCard, CommonDeck},
    tracker::{EventType, track_event},
};

// return a list of cards that match the filters
fn filter_cards<'a>(filter: &str, info: &'a CardsInfo) -> Vec<&'a CardEntry> {
    let filter = filter.to_lowercase();
    info.values()
        .flat_map(|cards| cards.iter())
        // TODO add more filter options
        .filter(|card| card.card_number.to_lowercase().contains(&filter))
        .collect()
}

#[component]
pub fn CardSearch(
    info: Signal<CardsInfo>,
    common_deck: Signal<CommonDeck>,
    is_edit: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
    }

    let mut cards = use_signal(Vec::new);
    let mut cards_filter = use_signal(String::new);
    const CARD_INCREMENT: usize = 120; // split even for 6, 5, 4, etc. columns
    let mut card_amount = use_signal(|| CARD_INCREMENT);
    let mut max_card_amount = use_signal(|| 0);
    let mut loading = use_signal(|| false);
    let mut tracking_sent_card_search: Signal<Option<Instant>> = use_signal(|| None);
    let mut tracking_sent_load_more: Signal<Option<Instant>> = use_signal(|| None);

    let update_filter = move |event: Event<FormData>| {
        let filter = event.value();
        *cards_filter.write() = filter.trim().to_lowercase();
        *card_amount.write() = CARD_INCREMENT;
        // scroll to top, after updating the filter, to show the first cards
        document().eval("document.getElementById('card_search_cards').scrollTop = 0;".into());

        if tracking_sent_card_search
            .peek()
            .as_ref()
            .map(|t| t.elapsed() >= Duration::from_secs(30))
            .unwrap_or(true)
        {
            track_event(
                EventType::EditDeck,
                EventData {
                    action: "Card search".into(),
                },
            );
            *tracking_sent_card_search.write() = Some(Instant::now());
        }
    };

    let _ = use_effect(move || {
        let filter = cards_filter.read();
        let _info = info.read();
        let _common_deck = common_deck.read();
        let filtered_cards = filter_cards(&filter, &_info);
        *max_card_amount.write() = filtered_cards.len();
        *cards.write() = filtered_cards
            .into_iter()
            // limit the number of cards shown
            .take(*card_amount.read())
            .map(move |card| {
                rsx! {
                    Card {
                        card: CommonCard {
                            manage_id: card.manage_id,
                            card_number: card.card_number.clone(),
                            amount: card
                                .manage_id
                                .and_then(|id| _common_deck.find_card(id))
                                .map(|c| c.amount)
                                .unwrap_or(0),
                        },
                        card_type: CardType::Main,
                        card_lang: use_signal(|| CardLanguage::Japanese),
                        is_preview: false,
                        info,
                        common_deck,
                        is_edit,
                    }
                }
            })
            .collect::<Vec<_>>();
        *loading.write() = false;
    });

    rsx! {
        div { class: "field",
            // TODO change label
            label { "for": "card_search", class: "label", "Card number" }
            div { class: "control",
                input {
                    id: "card_search",
                    class: "input",
                    r#type: "text",
                    oninput: update_filter,
                    maxlength: 100,
                    placeholder: "Search for a card...",
                }
            }
        }
        div {
            id: "card_search_cards",
            class: "block is-flex is-flex-wrap-wrap is-justify-content-center",
            style: "max-height: 50vh; overflow: scroll;",
            for card in cards.read().iter() {
                {card}
            }

            // load more cards
            if *card_amount.read() < *max_card_amount.read() {
                div {
                    class: "field m-2 is-flex is-justify-content-center",
                    style: "width: 100%",
                    div { class: "control",
                        button {
                            r#type: "button",
                            class: "button",
                            class: if *loading.read() { "is-loading" },
                            onclick: move |_| {
                                *loading.write() = true;
                                *card_amount.write() += CARD_INCREMENT;
                                if tracking_sent_load_more
                                    .peek()
                                    .as_ref()
                                    .map(|t| t.elapsed() >= Duration::from_secs(30))
                                    .unwrap_or(true)
                                {
                                    track_event(
                                        EventType::EditDeck,
                                        EventData {
                                            action: "Load more cards".into(),
                                        },
                                    );
                                    *tracking_sent_load_more.write() = Some(Instant::now());
                                }
                            },
                            span { class: "icon",
                                i { class: "fa-solid fa-arrow-down" }
                            }
                            span { "Load more cards" }
                        }
                    }
                }
            }
        }
    }
}
