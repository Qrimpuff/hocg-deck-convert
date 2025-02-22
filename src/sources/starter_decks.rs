use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::*};
use serde::Serialize;

use crate::{EventType, track_event};

use super::{CardsInfo, CommonCards, CommonDeck};

#[derive(Debug, Clone)]
struct DeckEntry {
    deck_id: String,
    display: String,
    deck: CommonDeck,
}

fn starter_decks(info: &CardsInfo) -> &'static Vec<DeckEntry> {
    static DECKS: OnceLock<Vec<DeckEntry>> = OnceLock::new();
    DECKS.get_or_init(|| {
        vec![
            // hSD01 - スタートデッキ「ときのそら&AZKi」(Sora oshi)
            DeckEntry {
                deck_id: "hSD01-001".into(),
                display: "hSD01 - Start Deck「Tokino Sora & AZKi」 (Sora oshi)".into(),
                deck: CommonDeck {
                    name: Some("Start Deck「Tokino Sora & AZKi」".into()),
                    oshi: CommonCards::from_card_number("hSD01-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD01-003".into(), 4, info),
                        CommonCards::from_card_number("hSD01-004".into(), 3, info),
                        CommonCards::from_card_number("hSD01-005".into(), 3, info),
                        CommonCards::from_card_number("hSD01-006".into(), 2, info),
                        CommonCards::from_card_number("hSD01-007".into(), 2, info),
                        CommonCards::from_card_number("hSD01-008".into(), 4, info),
                        CommonCards::from_card_number("hSD01-009".into(), 3, info),
                        CommonCards::from_card_number("hSD01-010".into(), 3, info),
                        CommonCards::from_card_number("hSD01-011".into(), 2, info),
                        CommonCards::from_card_number("hSD01-012".into(), 2, info),
                        CommonCards::from_card_number("hSD01-013".into(), 2, info),
                        CommonCards::from_card_number("hSD01-014".into(), 2, info),
                        CommonCards::from_card_number("hSD01-015".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 3, info),
                        CommonCards::from_card_number("hSD01-017".into(), 3, info),
                        CommonCards::from_card_number("hSD01-018".into(), 3, info),
                        CommonCards::from_card_number("hSD01-019".into(), 3, info),
                        CommonCards::from_card_number("hSD01-020".into(), 2, info),
                        CommonCards::from_card_number("hSD01-021".into(), 2, info),
                    ],
                    cheer_deck: vec![
                        CommonCards::from_card_number("hY01-001".into(), 10, info),
                        CommonCards::from_card_number("hY02-001".into(), 10, info),
                    ],
                },
            },
            // hSD01 - スタートデッキ「ときのそら&AZKi」(AZKi oshi)
            DeckEntry {
                deck_id: "hSD01-002".into(),
                display: "hSD01 - Start Deck「Tokino Sora & AZKi」 (AZKi oshi)".into(),
                deck: CommonDeck {
                    name: Some("Start Deck「Tokino Sora & AZKi」".into()),
                    oshi: CommonCards::from_card_number("hSD01-002".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD01-003".into(), 4, info),
                        CommonCards::from_card_number("hSD01-004".into(), 3, info),
                        CommonCards::from_card_number("hSD01-005".into(), 3, info),
                        CommonCards::from_card_number("hSD01-006".into(), 2, info),
                        CommonCards::from_card_number("hSD01-007".into(), 2, info),
                        CommonCards::from_card_number("hSD01-008".into(), 4, info),
                        CommonCards::from_card_number("hSD01-009".into(), 3, info),
                        CommonCards::from_card_number("hSD01-010".into(), 3, info),
                        CommonCards::from_card_number("hSD01-011".into(), 2, info),
                        CommonCards::from_card_number("hSD01-012".into(), 2, info),
                        CommonCards::from_card_number("hSD01-013".into(), 2, info),
                        CommonCards::from_card_number("hSD01-014".into(), 2, info),
                        CommonCards::from_card_number("hSD01-015".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 3, info),
                        CommonCards::from_card_number("hSD01-017".into(), 3, info),
                        CommonCards::from_card_number("hSD01-018".into(), 3, info),
                        CommonCards::from_card_number("hSD01-019".into(), 3, info),
                        CommonCards::from_card_number("hSD01-020".into(), 2, info),
                        CommonCards::from_card_number("hSD01-021".into(), 2, info),
                    ],
                    cheer_deck: vec![
                        CommonCards::from_card_number("hY01-001".into(), 10, info),
                        CommonCards::from_card_number("hY02-001".into(), 10, info),
                    ],
                },
            },
            // hSD02 - スタートデッキ 赤 百鬼あやめ
            DeckEntry {
                deck_id: "hSD02-001".into(),
                display: "hSD02 - Start Deck (Red) Nakiri Ayame".into(),
                deck: CommonDeck {
                    name: Some("hSD02 - Start Deck (Red) Nakiri Ayame".into()),
                    oshi: CommonCards::from_card_number("hSD02-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD02-002".into(), 6, info),
                        CommonCards::from_card_number("hSD02-003".into(), 4, info),
                        CommonCards::from_card_number("hSD02-004".into(), 2, info),
                        CommonCards::from_card_number("hSD02-005".into(), 4, info),
                        CommonCards::from_card_number("hSD02-006".into(), 4, info),
                        CommonCards::from_card_number("hSD02-007".into(), 2, info),
                        CommonCards::from_card_number("hSD02-008".into(), 2, info),
                        CommonCards::from_card_number("hSD02-009".into(), 2, info),
                        CommonCards::from_card_number("hSD02-010".into(), 2, info),
                        CommonCards::from_card_number("hSD02-011".into(), 2, info),
                        CommonCards::from_card_number("hSD02-012".into(), 4, info),
                        CommonCards::from_card_number("hSD02-013".into(), 2, info),
                        CommonCards::from_card_number("hSD02-014".into(), 2, info),
                        CommonCards::from_card_number("hBP01-104".into(), 1, info),
                        CommonCards::from_card_number("hBP01-108".into(), 1, info),
                        CommonCards::from_card_number("hSD01-016".into(), 4, info),
                        CommonCards::from_card_number("hSD01-017".into(), 2, info),
                        CommonCards::from_card_number("hSD01-018".into(), 4, info),
                    ],
                    cheer_deck: vec![CommonCards::from_card_number("hY03-001".into(), 20, info)],
                },
            },
            // hSD03 - スタートデッキ 青 猫又おかゆ
            DeckEntry {
                deck_id: "hSD03-001".into(),
                display: "hSD03 - Start Deck (Blue) Nekomata Okayu".into(),
                deck: CommonDeck {
                    name: Some("hSD03 - Start Deck (Blue) Nekomata Okayu".into()),
                    oshi: CommonCards::from_card_number("hSD03-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD03-002".into(), 6, info),
                        CommonCards::from_card_number("hSD03-003".into(), 4, info),
                        CommonCards::from_card_number("hSD03-004".into(), 2, info),
                        CommonCards::from_card_number("hSD03-005".into(), 4, info),
                        CommonCards::from_card_number("hSD03-006".into(), 4, info),
                        CommonCards::from_card_number("hSD03-007".into(), 2, info),
                        CommonCards::from_card_number("hSD03-008".into(), 2, info),
                        CommonCards::from_card_number("hSD03-009".into(), 2, info),
                        CommonCards::from_card_number("hSD03-010".into(), 2, info),
                        CommonCards::from_card_number("hSD03-011".into(), 2, info),
                        CommonCards::from_card_number("hSD03-012".into(), 4, info),
                        CommonCards::from_card_number("hSD03-013".into(), 2, info),
                        CommonCards::from_card_number("hSD03-014".into(), 2, info),
                        CommonCards::from_card_number("hBP01-105".into(), 2, info),
                        CommonCards::from_card_number("hBP01-108".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 4, info),
                        CommonCards::from_card_number("hSD01-017".into(), 2, info),
                        CommonCards::from_card_number("hSD01-019".into(), 2, info),
                    ],
                    cheer_deck: vec![CommonCards::from_card_number("hY04-001".into(), 20, info)],
                },
            },
            // hSD04 - スタートデッキ 紫 癒月ちょこ
            DeckEntry {
                deck_id: "hSD04-001".into(),
                display: "hSD04 - Start Deck (Purple) Yuzuki Choco".into(),
                deck: CommonDeck {
                    name: Some("hSD04 - Start Deck (Purple) Yuzuki Choco".into()),
                    oshi: CommonCards::from_card_number("hSD04-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD04-002".into(), 6, info),
                        CommonCards::from_card_number("hSD04-003".into(), 4, info),
                        CommonCards::from_card_number("hSD04-004".into(), 2, info),
                        CommonCards::from_card_number("hSD04-005".into(), 4, info),
                        CommonCards::from_card_number("hSD04-006".into(), 4, info),
                        CommonCards::from_card_number("hSD04-007".into(), 2, info),
                        CommonCards::from_card_number("hSD04-008".into(), 2, info),
                        CommonCards::from_card_number("hSD04-009".into(), 2, info),
                        CommonCards::from_card_number("hSD04-010".into(), 2, info),
                        CommonCards::from_card_number("hSD04-011".into(), 2, info),
                        CommonCards::from_card_number("hSD04-012".into(), 4, info),
                        CommonCards::from_card_number("hSD04-013".into(), 2, info),
                        CommonCards::from_card_number("hSD04-014".into(), 2, info),
                        CommonCards::from_card_number("hBP01-104".into(), 2, info),
                        CommonCards::from_card_number("hBP01-106".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 4, info),
                        CommonCards::from_card_number("hSD01-017".into(), 2, info),
                        CommonCards::from_card_number("hSD01-019".into(), 2, info),
                    ],
                    cheer_deck: vec![CommonCards::from_card_number("hY05-001".into(), 20, info)],
                },
            },
            // hSD05 - スタートデッキ 白 轟はじめ
            DeckEntry {
                deck_id: "hSD05-001".into(),
                display: "hSD05 - Start Deck (White)  Todoroki Hajime".into(),
                deck: CommonDeck {
                    name: Some("hSD05 - Start Deck (White)  Todoroki Hajime".into()),
                    oshi: CommonCards::from_card_number("hSD05-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD05-002".into(), 6, info),
                        CommonCards::from_card_number("hSD05-003".into(), 4, info),
                        CommonCards::from_card_number("hSD05-004".into(), 2, info),
                        CommonCards::from_card_number("hSD05-005".into(), 4, info),
                        CommonCards::from_card_number("hSD05-006".into(), 4, info),
                        CommonCards::from_card_number("hSD05-007".into(), 2, info),
                        CommonCards::from_card_number("hSD05-008".into(), 2, info),
                        CommonCards::from_card_number("hSD05-009".into(), 2, info),
                        CommonCards::from_card_number("hSD05-010".into(), 2, info),
                        CommonCards::from_card_number("hSD05-011".into(), 2, info),
                        CommonCards::from_card_number("hSD05-012".into(), 2, info),
                        CommonCards::from_card_number("hSD05-013".into(), 2, info),
                        CommonCards::from_card_number("hSD05-014".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 4, info),
                        CommonCards::from_card_number("hSD01-017".into(), 2, info),
                        CommonCards::from_card_number("hBP01-104".into(), 2, info),
                        CommonCards::from_card_number("hBP01-108".into(), 2, info),
                        CommonCards::from_card_number_and_index("hPR-002".into(), 2, 4, info),
                    ],
                    cheer_deck: vec![CommonCards::from_card_number("hY01-001".into(), 20, info)],
                },
            },
            // hSD06 - スタートデッキ 緑 風真いろは
            DeckEntry {
                deck_id: "hSD06-001".into(),
                display: "hSD06 - Start Deck (Green) Kazama Iroha".into(),
                deck: CommonDeck {
                    name: Some("hSD06 - Start Deck (Green) Kazama Iroha".into()),
                    oshi: CommonCards::from_card_number("hSD06-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number_and_index("hBP01-048".into(), 2, 6, info),
                        CommonCards::from_card_number("hSD06-002".into(), 4, info),
                        CommonCards::from_card_number("hSD06-003".into(), 2, info),
                        CommonCards::from_card_number("hSD06-004".into(), 4, info),
                        CommonCards::from_card_number("hSD06-005".into(), 4, info),
                        CommonCards::from_card_number_and_index("hBP01-050".into(), 2, 2, info),
                        CommonCards::from_card_number("hSD06-006".into(), 2, info),
                        CommonCards::from_card_number("hSD06-007".into(), 2, info),
                        CommonCards::from_card_number("hSD06-008".into(), 2, info),
                        CommonCards::from_card_number("hSD06-009".into(), 2, info),
                        CommonCards::from_card_number("hSD06-010".into(), 2, info),
                        CommonCards::from_card_number("hSD06-011".into(), 2, info),
                        CommonCards::from_card_number("hSD06-012".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 4, info),
                        CommonCards::from_card_number("hSD01-017".into(), 2, info),
                        CommonCards::from_card_number("hBP01-104".into(), 2, info),
                        CommonCards::from_card_number("hBP02-076".into(), 2, info),
                        CommonCards::from_card_number("hBP02-080".into(), 4, info),
                    ],
                    cheer_deck: vec![CommonCards::from_card_number("hY02-001".into(), 20, info)],
                },
            },
            // hSD07 - スタートデッキ 黄 不知火フレア
            DeckEntry {
                deck_id: "hSD07-001".into(),
                display: "hSD07 - Start Deck (Yellow) Shiranui Flare".into(),
                deck: CommonDeck {
                    name: Some("hSD07 - Start Deck (Yellow) Shiranui Flare".into()),
                    oshi: CommonCards::from_card_number("hSD07-001".into(), 1, info),
                    main_deck: vec![
                        CommonCards::from_card_number("hSD07-002".into(), 6, info),
                        CommonCards::from_card_number("hSD07-003".into(), 4, info),
                        CommonCards::from_card_number("hSD07-004".into(), 2, info),
                        CommonCards::from_card_number("hSD07-005".into(), 4, info),
                        CommonCards::from_card_number("hSD07-006".into(), 4, info),
                        CommonCards::from_card_number("hSD07-007".into(), 2, info),
                        CommonCards::from_card_number("hSD07-008".into(), 2, info),
                        CommonCards::from_card_number("hSD07-009".into(), 2, info),
                        CommonCards::from_card_number("hSD07-010".into(), 2, info),
                        CommonCards::from_card_number("hSD07-011".into(), 2, info),
                        CommonCards::from_card_number("hSD07-012".into(), 2, info),
                        CommonCards::from_card_number("hSD07-013".into(), 2, info),
                        CommonCards::from_card_number("hSD07-014".into(), 4, info),
                        CommonCards::from_card_number("hSD07-015".into(), 2, info),
                        CommonCards::from_card_number("hSD01-016".into(), 4, info),
                        CommonCards::from_card_number("hSD01-017".into(), 2, info),
                        CommonCards::from_card_number("hSD01-018".into(), 2, info),
                        CommonCards::from_card_number("hBP01-104".into(), 2, info),
                    ],
                    cheer_deck: vec![CommonCards::from_card_number("hY06-001".into(), 20, info)],
                },
            },
        ]
    })
}

#[component]
pub fn Import(
    mut common_deck: Signal<Option<CommonDeck>>,
    info: Signal<CardsInfo>,
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
            .and_then(|idx| starter_decks(&info.read()).get(*idx));

        debug!("{:?}", deck);
        if let Some(deck) = deck {
            track_event(
                EventType::Import("Starter deck".into()),
                EventData {
                    format: "Starter deck",
                    deck_id: deck.deck_id.clone(),
                },
            );
        }
        *common_deck.write() = deck.map(|d| d.deck.clone());

        *show_price.write() = false;
        *loading.write() = false;
    };

    // display once
    use_effect(move || {
        if common_deck.peek().is_none() {
            load_deck();
        }
    });

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
                        for (idx , deck) in starter_decks(&info.read()).iter().enumerate() {
                            option { value: "{idx}", "{deck.display}" }
                        }
                    }
                }
            }
        }
    }
}
