use std::collections::HashMap;

use dioxus::logger::tracing::debug;
use dioxus::prelude::*;
use serde::Serialize;

use crate::{EventType, components::card_search::CardSearch, sources::PartialDeck, track_event};

use super::{CardsInfo, CommonCard, CommonDeck, CommonDeckConversion};

#[component]
pub fn Import(
    mut common_deck: Signal<Option<CommonDeck>>,
    info: Signal<CardsInfo>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        game_title_id: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        deck_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let mut edit_deck = use_signal(|| {
        common_deck
            .read()
            .as_ref()
            .map(|d| PartialDeck::from_common_deck(d.clone()))
            .unwrap_or_default()
    });
    let edit_deck_name =
        use_memo(move || edit_deck.read().name.as_ref().cloned().unwrap_or_default());

    // update common deck when edit deck changes
    let _ = use_effect(move || {
        let deck = edit_deck.read();
        let deck = PartialDeck::to_common_deck(deck.clone());
        if let Some(deck) = deck {
            common_deck.write().replace(deck);
        }
    });

    let update_deck_name = move |event: Event<FormData>| {
        let deck_name = event.value();
        edit_deck.write().name = Some(deck_name.trim().to_string()).filter(|s| !s.is_empty());
    };

    rsx! {
        div { class: "field",
            label { "for": "edit_deck_name", class: "label", "Deck name" }
            div { class: "control",
                input {
                    id: "edit_deck_name",
                    class: "input",
                    r#type: "text",
                    oninput: update_deck_name,
                    maxlength: 100,
                    placeholder: "Enter a name...",
                    value: "{edit_deck_name}",
                }
            }
        }

        CardSearch { info, common_deck }
    }
}
