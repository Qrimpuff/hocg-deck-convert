use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::*};
use serde::Serialize;

use crate::{track_convert_event, EventType};

use super::{CardsInfoMap, CommonCards, CommonDeck};

#[derive(Debug, Clone)]
struct DeckEntry {
    deck_id: String,
    display: String,
    deck: CommonDeck,
}

fn starter_decks(map: &CardsInfoMap) -> &'static Vec<DeckEntry> {
    static DECKS: OnceLock<Vec<DeckEntry>> = OnceLock::new();
    DECKS.get_or_init(|| {
        vec![
            // hSD01 - スタートデッキ「ときのそら&AZKi」(Sora oshi)
            DeckEntry {
                deck_id: "hSD01-001".into(),
                display: "hSD01 - スタートデッキ「ときのそら&AZKi」 (Sora oshi)".into(),
                deck: CommonDeck {
                    name: Some("スタートデッキ「ときのそら&AZKi」".into()),
                    oshi: CommonCards::from_card_number("hSD01-001".into(), 1, map),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD01-003".into(), 4, map),
                        CommonCards::from_card_number("hSD01-004".into(), 3, map),
                        CommonCards::from_card_number("hSD01-005".into(), 3, map),
                        CommonCards::from_card_number("hSD01-006".into(), 2, map),
                        CommonCards::from_card_number("hSD01-007".into(), 2, map),
                        CommonCards::from_card_number("hSD01-008".into(), 4, map),
                        CommonCards::from_card_number("hSD01-009".into(), 3, map),
                        CommonCards::from_card_number("hSD01-010".into(), 3, map),
                        CommonCards::from_card_number("hSD01-011".into(), 2, map),
                        CommonCards::from_card_number("hSD01-012".into(), 2, map),
                        CommonCards::from_card_number("hSD01-013".into(), 2, map),
                        CommonCards::from_card_number("hSD01-014".into(), 2, map),
                        CommonCards::from_card_number("hSD01-015".into(), 2, map),
                        CommonCards::from_card_number("hSD01-016".into(), 3, map),
                        CommonCards::from_card_number("hSD01-017".into(), 3, map),
                        CommonCards::from_card_number("hSD01-018".into(), 3, map),
                        CommonCards::from_card_number("hSD01-019".into(), 3, map),
                        CommonCards::from_card_number("hSD01-020".into(), 2, map),
                        CommonCards::from_card_number("hSD01-021".into(), 2, map),
                    ],
                    cheer_deck: vec![
                        CommonCards::from_card_number("hY01-001".into(), 10, map),
                        CommonCards::from_card_number("hY02-001".into(), 10, map),
                    ],
                },
            },
            // hSD01 - スタートデッキ「ときのそら&AZKi」(AZKi oshi)
            DeckEntry {
                deck_id: "hSD01-002".into(),
                display: "hSD01 - スタートデッキ「ときのそら&AZKi」 (AZKi oshi)".into(),
                deck: CommonDeck {
                    name: Some("スタートデッキ「ときのそら&AZKi」".into()),
                    oshi: CommonCards::from_card_number("hSD01-002".into(), 1, map),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD01-003".into(), 4, map),
                        CommonCards::from_card_number("hSD01-004".into(), 3, map),
                        CommonCards::from_card_number("hSD01-005".into(), 3, map),
                        CommonCards::from_card_number("hSD01-006".into(), 2, map),
                        CommonCards::from_card_number("hSD01-007".into(), 2, map),
                        CommonCards::from_card_number("hSD01-008".into(), 4, map),
                        CommonCards::from_card_number("hSD01-009".into(), 3, map),
                        CommonCards::from_card_number("hSD01-010".into(), 3, map),
                        CommonCards::from_card_number("hSD01-011".into(), 2, map),
                        CommonCards::from_card_number("hSD01-012".into(), 2, map),
                        CommonCards::from_card_number("hSD01-013".into(), 2, map),
                        CommonCards::from_card_number("hSD01-014".into(), 2, map),
                        CommonCards::from_card_number("hSD01-015".into(), 2, map),
                        CommonCards::from_card_number("hSD01-016".into(), 3, map),
                        CommonCards::from_card_number("hSD01-017".into(), 3, map),
                        CommonCards::from_card_number("hSD01-018".into(), 3, map),
                        CommonCards::from_card_number("hSD01-019".into(), 3, map),
                        CommonCards::from_card_number("hSD01-020".into(), 2, map),
                        CommonCards::from_card_number("hSD01-021".into(), 2, map),
                    ],
                    cheer_deck: vec![
                        CommonCards::from_card_number("hY01-001".into(), 10, map),
                        CommonCards::from_card_number("hY02-001".into(), 10, map),
                    ],
                },
            },
        ]
    })
}

#[component]
pub fn Import(
    mut common_deck: Signal<Option<CommonDeck>>,
    map: Signal<CardsInfoMap>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        deck_id: String,
    }

    let mut starter_deck_idx: Signal<Option<usize>> = use_signal(|| Some(0));
    let mut loading = use_signal(|| false);

    let mut load_deck = move || {
        *loading.write() = true;

        let deck = starter_deck_idx
            .read()
            .as_ref()
            .and_then(|idx| starter_decks(&map.read()).get(*idx));

        debug!("{:?}", deck);
        if let Some(deck) = deck {
            track_convert_event(
                EventType::Import("Stater deck".into()),
                EventData {
                    format: "Stater deck",
                    deck_id: deck.deck_id.clone(),
                },
            );
        }
        *common_deck.write() = deck.map(|d| d.deck.clone());

        *show_price.write() = false;
        *loading.write() = false;
    };

    if common_deck.read().is_none() {
        load_deck();
    }

    rsx! {
        div { class: "field",
            label { "for": "starter_deck", class: "label", "Starter deck" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "starter_deck",
                        oninput: move |ev| {
                            *starter_deck_idx.write() = ev.value().parse().ok();
                            load_deck();
                        },
                        for (idx , deck) in starter_decks(&map.read()).iter().enumerate() {
                            option { value: "{idx}", "{deck.display}" }
                        }
                    }
                }
            }
        }
    }
}
