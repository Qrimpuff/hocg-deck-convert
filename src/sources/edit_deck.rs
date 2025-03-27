use dioxus::prelude::*;
use serde::Serialize;

use crate::components::card_search::CardSearch;

use super::{CardsInfo, CommonDeck};

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    info: Signal<CardsInfo>,
    is_edit: Signal<bool>,
    show_price: Signal<bool>,
) -> Element {
    // TODO update event data for edit
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

    let common_deck_name = use_memo(move || {
        common_deck
            .read()
            .name
            .as_ref()
            .cloned()
            .unwrap_or_default()
    });


    let update_deck_name = move |event: Event<FormData>| {
        let deck_name = event.value();
        common_deck.write().name = Some(deck_name.trim().to_string()).filter(|s| !s.is_empty());
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
                    value: "{common_deck_name}",
                }
            }
        }

        CardSearch { info, common_deck, is_edit }
    }
}
