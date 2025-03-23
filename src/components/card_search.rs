use dioxus::prelude::*;
use hocg_fan_sim_assets_model::{CardEntry, CardsInfo};

use crate::{
    CardLanguage, CardType,
    components::card::Card,
    sources::{CommonCard, CommonDeck, price_check::PriceCache},
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
pub fn CardSearch(info: Signal<CardsInfo>, common_deck: Signal<Option<CommonDeck>>) -> Element {
    let mut cards = use_signal(Vec::new);
    let mut cards_filter = use_signal(String::new);

    let update_filter = move |event: Event<FormData>| {
        let filter = event.value();
        *cards_filter.write() = filter.trim().to_lowercase();
    };

    let _ = use_effect(move || {
        let filter = cards_filter.read();
        let _info = info.read();
        let _common_deck = common_deck.read();
        // limit the number of cards shown (max 100?)
        // TODO maybe add a placeholder at the end of the list, to show that there are more cards
        if filter.len() >= 3 {
            *cards.write() = filter_cards(&filter, &_info)
                .into_iter()
                .take(100)
                .map(move |card| {
                    rsx! {
                        Card {
                            card: CommonCard {
                                manage_id: card.manage_id,
                                card_number: card.card_number.clone(),
                                amount: _common_deck
                                    .as_ref()
                                    .and_then(|d| card.manage_id.and_then(|id| d.find_card(id)))
                                    .map(|c| c.amount)
                                    .unwrap_or(0),
                            },
                            card_type: CardType::Main,
                            card_lang: use_signal(|| CardLanguage::Japanese),
                            info,
                            common_deck,
                        }
                    }
                })
                .collect::<Vec<_>>();
        } else {
            *cards.write() = Vec::new();
        }
    });

    rsx! {
        div { class: "field",
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
            class: "block is-flex is-flex-wrap-wrap is-justify-content-center",
            style: "max-height: 50vh; overflow: scroll;",
            for card in cards.read().iter() {
                {card}
            }
        }
    }
}
