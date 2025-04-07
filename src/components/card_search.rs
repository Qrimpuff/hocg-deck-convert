use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
use itertools::Itertools;
use serde::Serialize;

use crate::{
    CardLanguage, CardType,
    components::card::Card,
    sources::{CommonCard, CommonDeck},
    tracker::{EventType, track_event},
};

// return a list of cards that match the filters
fn filter_cards<'a>(filter: &str, db: &'a CardsDatabase) -> Vec<&'a hocg::CardIllustration> {
    let filter = filter.trim().to_lowercase();
    let filter = filter.split_whitespace().collect_vec();
    let mut cards = db
        .values()
        // filter by text
        .filter(|card| {
            // check that all words matches
            filter.iter().all(|filter| {
                card.card_number.to_lowercase().contains(filter)
                    || card.name.japanese.to_lowercase().contains(filter)
                    || card
                        .name
                        .english
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(filter))
                        .unwrap_or_default()
                    || format!("{:?}", card.card_type)
                        .to_lowercase()
                        .contains(filter)
                    || format!("{:?}", card.colors).to_lowercase().contains(filter)
                    || card.life.to_string().contains(filter)
                    || card.hp.to_string().contains(filter)
                    || format!("{:?}", card.bloom_level)
                        .to_lowercase()
                        .contains(filter)
                    || card
                        .buzz
                        .then_some("buzz")
                        .unwrap_or_default()
                        .contains(filter)
                    || card
                        .limited
                        .then_some("limited")
                        .unwrap_or_default()
                        .contains(filter)
                    || card.text.japanese.to_lowercase().contains(filter)
                    || card
                        .text
                        .english
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(filter))
                        .unwrap_or_default()
                    || card
                        .tags
                        .iter()
                        .any(|tag| tag.japanese.to_lowercase().contains(filter))
                    || card.tags.iter().any(|tag| {
                        tag.english
                            .as_ref()
                            .map(|t| t.to_lowercase().contains(filter))
                            .unwrap_or_default()
                    })
            })
        })
        // TODO add more filter options (from select boxes)
        .collect_vec();

    // TODO sort by relevance
    cards.sort();

    cards
        .into_iter()
        .flat_map(|card| &card.illustrations)
        .collect()
}

#[component]
pub fn CardSearch(
    db: Signal<CardsDatabase>,
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

    let update_filter = move |event: Event<FormData>| {
        let filter = event.value();
        *cards_filter.write() = filter.trim().to_lowercase();
        *card_amount.write() = CARD_INCREMENT;
        // scroll to top, after updating the filter, to show the first cards
        document().eval("document.getElementById('card_search_cards').scrollTop = 0;".into());

        track_event(
            EventType::EditDeck,
            EventData {
                action: "Card search".into(),
            },
        );
    };

    let _ = use_effect(move || {
        let filter = cards_filter.read();
        let _db = db.read();
        let _common_deck = common_deck.read();
        let filtered_cards = filter_cards(&filter, &_db);
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
                        db,
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
            label { "for": "card_search", class: "label", "Card search" }
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
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Load more cards".into(),
                                    },
                                );
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
