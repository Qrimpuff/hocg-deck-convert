use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::*};
use serde::Serialize;

use crate::{EventType, track_event};

use super::{CardsInfo, CommonCard, CommonDeck};

#[derive(Debug, Clone)]
struct DeckEntry {
    deck_id: String,
    display: String,
    deck: CommonDeck,
}

fn card(card_number: &str, manage_id: u32, amount: u32, info: &CardsInfo) -> CommonCard {
    CommonCard::from_card_number_and_manage_id(card_number.into(), manage_id, amount, info)
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
                    oshi: card("hSD01-001", 1, 1, info),
                    main_deck: vec![
                        card("hSD01-003", 3, 4, info),
                        card("hSD01-004", 4, 3, info),
                        card("hSD01-005", 5, 3, info),
                        card("hSD01-006", 6, 2, info),
                        card("hSD01-007", 7, 2, info),
                        card("hSD01-008", 8, 4, info),
                        card("hSD01-009", 9, 3, info),
                        card("hSD01-010", 10, 3, info),
                        card("hSD01-011", 11, 2, info),
                        card("hSD01-012", 12, 2, info),
                        card("hSD01-013", 13, 2, info),
                        card("hSD01-014", 14, 2, info),
                        card("hSD01-015", 15, 2, info),
                        card("hSD01-016", 16, 3, info),
                        card("hSD01-017", 17, 3, info),
                        card("hSD01-018", 18, 3, info),
                        card("hSD01-019", 19, 3, info),
                        card("hSD01-020", 20, 2, info),
                        card("hSD01-021", 21, 2, info),
                    ],
                    cheer_deck: vec![
                        card("hY01-001", 168, 10, info),
                        card("hY02-001", 169, 10, info),
                    ],
                },
            },
            // hSD01 - スタートデッキ「ときのそら&AZKi」(AZKi oshi)
            DeckEntry {
                deck_id: "hSD01-002".into(),
                display: "hSD01 - Start Deck「Tokino Sora & AZKi」 (AZKi oshi)".into(),
                deck: CommonDeck {
                    name: Some("Start Deck「Tokino Sora & AZKi」".into()),
                    oshi: card("hSD01-002", 2, 1, info),
                    main_deck: vec![
                        card("hSD01-003", 3, 4, info),
                        card("hSD01-004", 4, 3, info),
                        card("hSD01-005", 5, 3, info),
                        card("hSD01-006", 6, 2, info),
                        card("hSD01-007", 7, 2, info),
                        card("hSD01-008", 8, 4, info),
                        card("hSD01-009", 9, 3, info),
                        card("hSD01-010", 10, 3, info),
                        card("hSD01-011", 11, 2, info),
                        card("hSD01-012", 12, 2, info),
                        card("hSD01-013", 13, 2, info),
                        card("hSD01-014", 14, 2, info),
                        card("hSD01-015", 15, 2, info),
                        card("hSD01-016", 16, 3, info),
                        card("hSD01-017", 17, 3, info),
                        card("hSD01-018", 18, 3, info),
                        card("hSD01-019", 19, 3, info),
                        card("hSD01-020", 20, 2, info),
                        card("hSD01-021", 21, 2, info),
                    ],
                    cheer_deck: vec![
                        card("hY01-001", 168, 10, info),
                        card("hY02-001", 169, 10, info),
                    ],
                },
            },
            // hSD02 - スタートデッキ 赤 百鬼あやめ
            DeckEntry {
                deck_id: "hSD02-001".into(),
                display: "hSD02 - Start Deck (Red) Nakiri Ayame".into(),
                deck: CommonDeck {
                    name: Some("hSD02 - Start Deck (Red) Nakiri Ayame".into()),
                    oshi: card("hSD02-001", 225, 1, info),
                    main_deck: vec![
                        card("hSD02-002", 226, 6, info),
                        card("hSD02-003", 227, 4, info),
                        card("hSD02-004", 228, 2, info),
                        card("hSD02-005", 229, 4, info),
                        card("hSD02-006", 230, 4, info),
                        card("hSD02-007", 231, 2, info),
                        card("hSD02-008", 232, 2, info),
                        card("hSD02-009", 233, 2, info),
                        card("hSD02-010", 234, 2, info),
                        card("hSD02-011", 235, 2, info),
                        card("hSD02-012", 236, 4, info),
                        card("hSD02-013", 237, 2, info),
                        card("hSD02-014", 238, 2, info),
                        card("hBP01-104", 145, 1, info),
                        card("hBP01-108", 149, 1, info),
                        card("hSD01-016", 16, 4, info),
                        card("hSD01-017", 17, 2, info),
                        card("hSD01-018", 18, 4, info),
                    ],
                    cheer_deck: vec![card("hY03-001", 170, 20, info)],
                },
            },
            // hSD03 - スタートデッキ 青 猫又おかゆ
            DeckEntry {
                deck_id: "hSD03-001".into(),
                display: "hSD03 - Start Deck (Blue) Nekomata Okayu".into(),
                deck: CommonDeck {
                    name: Some("hSD03 - Start Deck (Blue) Nekomata Okayu".into()),
                    oshi: card("hSD03-001", 239, 1, info),
                    main_deck: vec![
                        card("hSD03-002", 240, 6, info),
                        card("hSD03-003", 241, 4, info),
                        card("hSD03-004", 242, 2, info),
                        card("hSD03-005", 243, 4, info),
                        card("hSD03-006", 244, 4, info),
                        card("hSD03-007", 245, 2, info),
                        card("hSD03-008", 246, 2, info),
                        card("hSD03-009", 247, 2, info),
                        card("hSD03-010", 248, 2, info),
                        card("hSD03-011", 249, 2, info),
                        card("hSD03-012", 250, 4, info),
                        card("hSD03-013", 251, 2, info),
                        card("hSD03-014", 252, 2, info),
                        card("hBP01-105", 146, 2, info),
                        card("hBP01-108", 149, 2, info),
                        card("hSD01-016", 16, 4, info),
                        card("hSD01-017", 17, 2, info),
                        card("hSD01-019", 19, 2, info),
                    ],
                    cheer_deck: vec![card("hY04-001", 171, 20, info)],
                },
            },
            // hSD04 - スタートデッキ 紫 癒月ちょこ
            DeckEntry {
                deck_id: "hSD04-001".into(),
                display: "hSD04 - Start Deck (Purple) Yuzuki Choco".into(),
                deck: CommonDeck {
                    name: Some("hSD04 - Start Deck (Purple) Yuzuki Choco".into()),
                    oshi: card("hSD04-001", 253, 1, info),
                    main_deck: vec![
                        card("hSD04-002", 254, 6, info),
                        card("hSD04-003", 255, 4, info),
                        card("hSD04-004", 256, 2, info),
                        card("hSD04-005", 257, 4, info),
                        card("hSD04-006", 258, 4, info),
                        card("hSD04-007", 259, 2, info),
                        card("hSD04-008", 260, 2, info),
                        card("hSD04-009", 261, 2, info),
                        card("hSD04-010", 262, 2, info),
                        card("hSD04-011", 263, 2, info),
                        card("hSD04-012", 264, 4, info),
                        card("hSD04-013", 265, 2, info),
                        card("hSD04-014", 266, 2, info),
                        card("hBP01-104", 145, 2, info),
                        card("hBP01-106", 147, 2, info),
                        card("hSD01-016", 16, 4, info),
                        card("hSD01-017", 17, 2, info),
                        card("hSD01-019", 19, 2, info),
                    ],
                    cheer_deck: vec![card("hY05-001", 267, 20, info)],
                },
            },
            // hSD05 - スタートデッキ 白 轟はじめ
            DeckEntry {
                deck_id: "hSD05-001".into(),
                display: "hSD05 - Start Deck (White) Todoroki Hajime".into(),
                deck: CommonDeck {
                    name: Some("hSD05 - Start Deck (White) Todoroki Hajime".into()),
                    oshi: card("hSD05-001", 517, 1, info),
                    main_deck: vec![
                        card("hSD05-002", 518, 6, info),
                        card("hSD05-003", 519, 4, info),
                        card("hSD05-004", 520, 2, info),
                        card("hSD05-005", 521, 4, info),
                        card("hSD05-006", 522, 4, info),
                        card("hSD05-007", 523, 2, info),
                        card("hSD05-008", 524, 2, info),
                        card("hSD05-009", 525, 2, info),
                        card("hSD05-010", 526, 2, info),
                        card("hSD05-011", 527, 2, info),
                        card("hSD05-012", 528, 2, info),
                        card("hSD05-013", 529, 2, info),
                        card("hSD05-014", 531, 2, info),
                        card("hSD01-016", 16, 4, info),
                        card("hSD01-017", 17, 2, info),
                        card("hBP01-104", 145, 2, info),
                        card("hBP01-108", 149, 2, info),
                        card("hPR-002", 530, 4, info),
                    ],
                    cheer_deck: vec![card("hY01-001", 168, 20, info)],
                },
            },
            // hSD06 - スタートデッキ 緑 風真いろは
            DeckEntry {
                deck_id: "hSD06-001".into(),
                display: "hSD06 - Start Deck (Green) Kazama Iroha".into(),
                deck: CommonDeck {
                    name: Some("hSD06 - Start Deck (Green) Kazama Iroha".into()),
                    oshi: card("hSD06-001", 532, 1, info),
                    main_deck: vec![
                        card("hBP01-048", 533, 6, info),
                        card("hSD06-002", 534, 4, info),
                        card("hSD06-003", 535, 2, info),
                        card("hSD06-004", 536, 4, info),
                        card("hSD06-005", 537, 4, info),
                        card("hBP01-050", 538, 2, info),
                        card("hSD06-006", 539, 2, info),
                        card("hSD06-007", 540, 2, info),
                        card("hSD06-008", 541, 2, info),
                        card("hSD06-009", 542, 2, info),
                        card("hSD06-010", 543, 2, info),
                        card("hSD06-011", 544, 2, info),
                        card("hSD06-012", 545, 2, info),
                        card("hSD01-016", 16, 4, info),
                        card("hSD01-017", 17, 2, info),
                        card("hBP01-104", 145, 2, info),
                        card("hBP02-076", 343, 2, info),
                        card("hBP02-080", 347, 4, info),
                    ],
                    cheer_deck: vec![card("hY02-001", 169, 20, info)],
                },
            },
            // hSD07 - スタートデッキ 黄 不知火フレア
            DeckEntry {
                deck_id: "hSD07-001".into(),
                display: "hSD07 - Start Deck (Yellow) Shiranui Flare".into(),
                deck: CommonDeck {
                    name: Some("hSD07 - Start Deck (Yellow) Shiranui Flare".into()),
                    oshi: card("hSD07-001", 546, 1, info),
                    main_deck: vec![
                        card("hSD07-002", 547, 6, info),
                        card("hSD07-003", 548, 4, info),
                        card("hSD07-004", 549, 2, info),
                        card("hSD07-005", 550, 4, info),
                        card("hSD07-006", 551, 4, info),
                        card("hSD07-007", 552, 2, info),
                        card("hSD07-008", 553, 2, info),
                        card("hSD07-009", 554, 2, info),
                        card("hSD07-010", 555, 2, info),
                        card("hSD07-011", 556, 2, info),
                        card("hSD07-012", 557, 2, info),
                        card("hSD07-013", 558, 2, info),
                        card("hSD07-014", 559, 4, info),
                        card("hSD07-015", 560, 2, info),
                        card("hSD01-016", 16, 4, info),
                        card("hSD01-017", 17, 2, info),
                        card("hSD01-018", 18, 2, info),
                        card("hBP01-104", 145, 2, info),
                    ],
                    cheer_deck: vec![card("hY06-001", 561, 20, info)],
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
