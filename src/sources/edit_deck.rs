use dioxus::prelude::*;
use serde::Serialize;

use crate::{
    components::card_search::CardSearch,
    tracker::{EventType, track_event},
};

use super::{CardsDatabase, CommonDeck};

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
    is_edit: Signal<bool>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
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

        track_event(
            EventType::EditDeck,
            EventData {
                action: "Update deck name".into(),
            },
        );
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

        CardSearch { db, common_deck, is_edit }
    }
}
