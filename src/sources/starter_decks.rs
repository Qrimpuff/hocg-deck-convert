use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::*};
use serde::Serialize;

use crate::{EventType, track_event};

use super::{CardsDatabase, CommonCard, CommonDeck};

#[derive(Debug, Clone)]
struct DeckEntry {
    deck_id: String,
    display: String,
    oshi_options: Option<Vec<CommonCard>>,
    deck: CommonDeck,
}

fn card(card_number: &str, manage_id: u32, amount: u32, db: &CardsDatabase) -> CommonCard {
    CommonCard::from_card_number_and_manage_id(card_number.into(), manage_id, amount, db)
}

fn starter_decks(db: &CardsDatabase) -> &'static Vec<DeckEntry> {
    static DECKS: OnceLock<Vec<DeckEntry>> = OnceLock::new();
    DECKS.get_or_init(|| {
        vec![
            // hSD01 - スタートデッキ「ときのそら&AZKi」
            DeckEntry {
                deck_id: "hSD01-001".into(),
                display: "hSD01 - Start Deck「Tokino Sora & AZKi」".into(),
                oshi_options: Some(vec![
                    card("hSD01-001", 1, 1, db),
                    card("hSD01-002", 2, 1, db),
                ]),
                deck: CommonDeck {
                    name: Some("Start Deck「Tokino Sora & AZKi」".into()),
                    oshi: Some(card("hSD01-001", 1, 1, db)),
                    main_deck: vec![
                        card("hSD01-003", 3, 4, db),
                        card("hSD01-004", 4, 3, db),
                        card("hSD01-005", 5, 3, db),
                        card("hSD01-006", 6, 2, db),
                        card("hSD01-007", 7, 2, db),
                        card("hSD01-008", 8, 4, db),
                        card("hSD01-009", 9, 3, db),
                        card("hSD01-010", 10, 3, db),
                        card("hSD01-011", 11, 2, db),
                        card("hSD01-012", 12, 2, db),
                        card("hSD01-013", 13, 2, db),
                        card("hSD01-014", 14, 2, db),
                        card("hSD01-015", 15, 2, db),
                        card("hSD01-016", 16, 3, db),
                        card("hSD01-017", 17, 3, db),
                        card("hSD01-018", 18, 3, db),
                        card("hSD01-019", 19, 3, db),
                        card("hSD01-020", 20, 2, db),
                        card("hSD01-021", 21, 2, db),
                    ],
                    cheer_deck: vec![card("hY01-001", 168, 10, db), card("hY02-001", 169, 10, db)],
                },
            },
            // hSD02 - スタートデッキ 赤 百鬼あやめ
            DeckEntry {
                deck_id: "hSD02-001".into(),
                display: "hSD02 - Start Deck (Red) Nakiri Ayame".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Red) Nakiri Ayame".into()),
                    oshi: Some(card("hSD02-001", 225, 1, db)),
                    main_deck: vec![
                        card("hSD02-002", 226, 6, db),
                        card("hSD02-003", 227, 4, db),
                        card("hSD02-004", 228, 2, db),
                        card("hSD02-005", 229, 4, db),
                        card("hSD02-006", 230, 4, db),
                        card("hSD02-007", 231, 2, db),
                        card("hSD02-008", 232, 2, db),
                        card("hSD02-009", 233, 2, db),
                        card("hSD02-010", 234, 2, db),
                        card("hSD02-011", 235, 2, db),
                        card("hSD02-012", 236, 4, db),
                        card("hSD02-013", 237, 2, db),
                        card("hSD02-014", 238, 2, db),
                        card("hBP01-104", 145, 1, db),
                        card("hBP01-108", 149, 1, db),
                        card("hSD01-016", 16, 4, db),
                        card("hSD01-017", 17, 2, db),
                        card("hSD01-018", 18, 4, db),
                    ],
                    cheer_deck: vec![card("hY03-001", 170, 20, db)],
                },
            },
            // hSD03 - スタートデッキ 青 猫又おかゆ
            DeckEntry {
                deck_id: "hSD03-001".into(),
                display: "hSD03 - Start Deck (Blue) Nekomata Okayu".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Blue) Nekomata Okayu".into()),
                    oshi: Some(card("hSD03-001", 239, 1, db)),
                    main_deck: vec![
                        card("hSD03-002", 240, 6, db),
                        card("hSD03-003", 241, 4, db),
                        card("hSD03-004", 242, 2, db),
                        card("hSD03-005", 243, 4, db),
                        card("hSD03-006", 244, 4, db),
                        card("hSD03-007", 245, 2, db),
                        card("hSD03-008", 246, 2, db),
                        card("hSD03-009", 247, 2, db),
                        card("hSD03-010", 248, 2, db),
                        card("hSD03-011", 249, 2, db),
                        card("hSD03-012", 250, 4, db),
                        card("hSD03-013", 251, 2, db),
                        card("hSD03-014", 252, 2, db),
                        card("hBP01-105", 146, 2, db),
                        card("hBP01-108", 149, 2, db),
                        card("hSD01-016", 16, 4, db),
                        card("hSD01-017", 17, 2, db),
                        card("hSD01-019", 19, 2, db),
                    ],
                    cheer_deck: vec![card("hY04-001", 171, 20, db)],
                },
            },
            // hSD04 - スタートデッキ 紫 癒月ちょこ
            DeckEntry {
                deck_id: "hSD04-001".into(),
                display: "hSD04 - Start Deck (Purple) Yuzuki Choco".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Purple) Yuzuki Choco".into()),
                    oshi: Some(card("hSD04-001", 253, 1, db)),
                    main_deck: vec![
                        card("hSD04-002", 254, 6, db),
                        card("hSD04-003", 255, 4, db),
                        card("hSD04-004", 256, 2, db),
                        card("hSD04-005", 257, 4, db),
                        card("hSD04-006", 258, 4, db),
                        card("hSD04-007", 259, 2, db),
                        card("hSD04-008", 260, 2, db),
                        card("hSD04-009", 261, 2, db),
                        card("hSD04-010", 262, 2, db),
                        card("hSD04-011", 263, 2, db),
                        card("hSD04-012", 264, 4, db),
                        card("hSD04-013", 265, 2, db),
                        card("hSD04-014", 266, 2, db),
                        card("hBP01-104", 145, 2, db),
                        card("hBP01-106", 147, 2, db),
                        card("hSD01-016", 16, 4, db),
                        card("hSD01-017", 17, 2, db),
                        card("hSD01-019", 19, 2, db),
                    ],
                    cheer_deck: vec![card("hY05-001", 267, 20, db)],
                },
            },
            // hSD05 - スタートデッキ 白 轟はじめ
            DeckEntry {
                deck_id: "hSD05-001".into(),
                display: "hSD05 - Start Deck (White) Todoroki Hajime".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (White) Todoroki Hajime".into()),
                    oshi: Some(card("hSD05-001", 517, 1, db)),
                    main_deck: vec![
                        card("hSD05-002", 518, 6, db),
                        card("hSD05-003", 519, 4, db),
                        card("hSD05-004", 520, 2, db),
                        card("hSD05-005", 521, 4, db),
                        card("hSD05-006", 522, 4, db),
                        card("hSD05-007", 523, 2, db),
                        card("hSD05-008", 524, 2, db),
                        card("hSD05-009", 525, 2, db),
                        card("hSD05-010", 526, 2, db),
                        card("hSD05-011", 527, 2, db),
                        card("hSD05-012", 528, 2, db),
                        card("hSD05-013", 529, 2, db),
                        card("hSD05-014", 531, 2, db),
                        card("hSD01-016", 16, 4, db),
                        card("hSD01-017", 17, 2, db),
                        card("hBP01-104", 145, 2, db),
                        card("hBP01-108", 149, 2, db),
                        card("hPR-002", 530, 4, db),
                    ],
                    cheer_deck: vec![card("hY01-001", 168, 20, db)],
                },
            },
            // hSD06 - スタートデッキ 緑 風真いろは
            DeckEntry {
                deck_id: "hSD06-001".into(),
                display: "hSD06 - Start Deck (Green) Kazama Iroha".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Green) Kazama Iroha".into()),
                    oshi: Some(card("hSD06-001", 532, 1, db)),
                    main_deck: vec![
                        card("hBP01-048", 533, 6, db),
                        card("hSD06-002", 534, 4, db),
                        card("hSD06-003", 535, 2, db),
                        card("hSD06-004", 536, 4, db),
                        card("hSD06-005", 537, 4, db),
                        card("hBP01-050", 538, 2, db),
                        card("hSD06-006", 539, 2, db),
                        card("hSD06-007", 540, 2, db),
                        card("hSD06-008", 541, 2, db),
                        card("hSD06-009", 542, 2, db),
                        card("hSD06-010", 543, 2, db),
                        card("hSD06-011", 544, 2, db),
                        card("hSD06-012", 545, 2, db),
                        card("hSD01-016", 16, 4, db),
                        card("hSD01-017", 17, 2, db),
                        card("hBP01-104", 145, 2, db),
                        card("hBP02-076", 343, 2, db),
                        card("hBP02-080", 347, 4, db),
                    ],
                    cheer_deck: vec![card("hY02-001", 169, 20, db)],
                },
            },
            // hSD07 - スタートデッキ 黄 不知火フレア
            DeckEntry {
                deck_id: "hSD07-001".into(),
                display: "hSD07 - Start Deck (Yellow) Shiranui Flare".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Yellow) Shiranui Flare".into()),
                    oshi: Some(card("hSD07-001", 546, 1, db)),
                    main_deck: vec![
                        card("hSD07-002", 547, 6, db),
                        card("hSD07-003", 548, 4, db),
                        card("hSD07-004", 549, 2, db),
                        card("hSD07-005", 550, 4, db),
                        card("hSD07-006", 551, 4, db),
                        card("hSD07-007", 552, 2, db),
                        card("hSD07-008", 553, 2, db),
                        card("hSD07-009", 554, 2, db),
                        card("hSD07-010", 555, 2, db),
                        card("hSD07-011", 556, 2, db),
                        card("hSD07-012", 557, 2, db),
                        card("hSD07-013", 558, 2, db),
                        card("hSD07-014", 559, 4, db),
                        card("hSD07-015", 560, 2, db),
                        card("hSD01-016", 16, 4, db),
                        card("hSD01-017", 17, 2, db),
                        card("hSD01-018", 18, 2, db),
                        card("hBP01-104", 145, 2, db),
                    ],
                    cheer_deck: vec![card("hY06-001", 561, 20, db)],
                },
            },
        ]
    })
}

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        deck_id: String,
    }

    let mut starter_deck_idx: Signal<Option<usize>> = use_signal(|| Some(0));
    let mut oshi_option_idx: Signal<Option<usize>> = use_signal(|| Some(0));
    let mut loading = use_signal(|| false);

    let oshi_options = use_memo(move || {
        starter_deck_idx
            .read()
            .as_ref()
            .and_then(|idx| starter_decks(&db.read()).get(*idx))
            .and_then(|d| d.oshi_options.as_ref())
    });

    let mut load_deck = move || {
        *loading.write() = true;

        let deck = starter_deck_idx
            .read()
            .as_ref()
            .and_then(|idx| starter_decks(&db.read()).get(*idx));

        debug!("{:?}", deck);
        if let Some(deck) = deck {
            track_event(
                EventType::Import("Starter deck".into()),
                EventData {
                    format: "Starter deck",
                    // the deck id is the oshi card number
                    deck_id: deck.deck_id.clone(),
                },
            );
        }
        *common_deck.write() = deck.map(|d| d.deck.clone()).unwrap_or_default();

        *show_price.write() = false;
        *loading.write() = false;
    };

    let mut change_oshi = move || {
        let oshi = oshi_options
            .read()
            .as_ref()
            .and_then(|o| oshi_option_idx.read().and_then(|idx| o.get(idx)));

        debug!("{:?}", oshi);
        if let Some(oshi) = oshi {
            let mut deck = common_deck.write();
            deck.add_card(oshi.clone(), crate::CardType::Oshi, &db.read());

            track_event(
                EventType::Import("Starter deck".into()),
                EventData {
                    format: "Starter deck",
                    // the deck id is the oshi card number
                    deck_id: oshi.card_number.clone(),
                },
            );
        }
    };

    // display once
    use_effect(move || {
        if *common_deck.peek() == Default::default() {
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
                        for (idx , deck) in starter_decks(&db.read()).iter().enumerate() {
                            option { value: "{idx}", "{deck.display}" }
                        }
                    }
                }
            }
        }

        if let Some(oshi_options) = *oshi_options.read() {
            div { class: "field",
                label { "for": "oshi_option", class: "label", "Oshi" }
                div { class: "control",
                    div { class: "select",
                        select {
                            id: "oshi_option",
                            oninput: move |ev| {
                                *oshi_option_idx.write() = ev.value().parse().ok();
                                change_oshi();
                            },
                            for (idx , oshi) in oshi_options.iter().enumerate() {
                                // TODO use card name with card number
                                option { value: "{idx}", "{oshi.card_number}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
