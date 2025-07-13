use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::*};
use serde::Serialize;

use crate::{CardLanguage::*, EventType, track_event, *};

use super::{CardsDatabase, CommonCard, CommonDeck};

#[derive(Debug, Clone)]
struct DeckEntry {
    deck_id: String,
    display: String,
    oshi_options: Option<Vec<CommonCard>>,
    deck: CommonDeck,
}

fn card(
    card_number: &str,
    manage_id: (CardLanguage, u32),
    amount: u32,
    db: &CardsDatabase,
) -> CommonCard {
    CommonCard::from_card_number_and_manage_id(card_number.into(), manage_id, amount, db)
}

fn starter_decks(db: &CardsDatabase) -> &'static Vec<DeckEntry> {
    static DECKS: OnceLock<Vec<DeckEntry>> = OnceLock::new();
    DECKS.get_or_init(|| {
        vec![
            // hSD01 - スタートデッキ「ときのそら&AZKi」
            DeckEntry {
                deck_id: "hSD01-001 JP".into(),
                display: "hSD01 - Start Deck「Tokino Sora & AZKi」(JP)".into(),
                oshi_options: Some(vec![
                    card("hSD01-001", (Japanese, 1), 1, db),
                    card("hSD01-002", (Japanese, 2), 1, db),
                ]),
                deck: CommonDeck {
                    name: Some("Start Deck「Tokino Sora & AZKi」".into()),
                    oshi: Some(card("hSD01-001", (Japanese, 1), 1, db)),
                    main_deck: vec![
                        card("hSD01-003", (Japanese, 3), 4, db),
                        card("hSD01-004", (Japanese, 4), 3, db),
                        card("hSD01-005", (Japanese, 5), 3, db),
                        card("hSD01-006", (Japanese, 6), 2, db),
                        card("hSD01-007", (Japanese, 7), 2, db),
                        card("hSD01-008", (Japanese, 8), 4, db),
                        card("hSD01-009", (Japanese, 9), 3, db),
                        card("hSD01-010", (Japanese, 10), 3, db),
                        card("hSD01-011", (Japanese, 11), 2, db),
                        card("hSD01-012", (Japanese, 12), 2, db),
                        card("hSD01-013", (Japanese, 13), 2, db),
                        card("hSD01-014", (Japanese, 14), 2, db),
                        card("hSD01-015", (Japanese, 15), 2, db),
                        card("hSD01-016", (Japanese, 16), 3, db),
                        card("hSD01-017", (Japanese, 17), 3, db),
                        card("hSD01-018", (Japanese, 18), 3, db),
                        card("hSD01-019", (Japanese, 19), 3, db),
                        card("hSD01-020", (Japanese, 20), 2, db),
                        card("hSD01-021", (Japanese, 21), 2, db),
                    ],
                    cheer_deck: vec![
                        card("hY01-001", (Japanese, 168), 10, db),
                        card("hY02-001", (Japanese, 169), 10, db),
                    ],
                },
            },
            // hSD01 - Start Deck – Tokino Sora & AZKi
            DeckEntry {
                deck_id: "hSD01-001 EN".into(),
                display: "hSD01 - Start Deck – Tokino Sora & AZKi (EN)".into(),
                oshi_options: Some(vec![
                    card("hSD01-001", (English, 1), 1, db),
                    card("hSD01-002", (English, 2), 1, db),
                ]),
                deck: CommonDeck {
                    name: Some("Start Deck – Tokino Sora & AZKi".into()),
                    oshi: Some(card("hSD01-001", (English, 1), 1, db)),
                    main_deck: vec![
                        card("hBP01-021", (English, 3), 4, db),
                        card("hSD01-004", (English, 4), 3, db),
                        card("hSD01-005", (English, 5), 3, db),
                        card("hSD01-006", (English, 6), 2, db),
                        card("hSD01-007", (English, 7), 2, db),
                        card("hBP01-044", (English, 8), 4, db),
                        card("hSD01-009", (English, 9), 3, db),
                        card("hSD01-010", (English, 10), 3, db),
                        card("hSD01-011", (English, 11), 2, db),
                        card("hSD01-012", (English, 12), 2, db),
                        card("hSD01-013", (English, 13), 2, db),
                        card("hSD01-014", (English, 14), 2, db),
                        card("hSD01-015", (English, 15), 2, db),
                        card("hSD01-016", (English, 16), 3, db),
                        card("hSD01-017", (English, 17), 3, db),
                        card("hBP01-104", (English, 18), 2, db),
                        card("hSD01-018", (English, 19), 2, db),
                        card("hSD01-019", (English, 20), 2, db),
                        card("hSD01-020", (English, 21), 2, db),
                        card("hSD01-021", (English, 22), 2, db),
                    ],
                    cheer_deck: vec![
                        card("hY01-001", (English, 23), 10, db),
                        card("hY02-001", (English, 24), 10, db),
                    ],
                },
            },
            // hSD02 - スタートデッキ 赤 百鬼あやめ
            DeckEntry {
                deck_id: "hSD02-001".into(),
                display: "hSD02 - Start Deck (Red) Nakiri Ayame".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Red) Nakiri Ayame".into()),
                    oshi: Some(card("hSD02-001", (Japanese, 225), 1, db)),
                    main_deck: vec![
                        card("hSD02-002", (Japanese, 226), 6, db),
                        card("hSD02-003", (Japanese, 227), 4, db),
                        card("hSD02-004", (Japanese, 228), 2, db),
                        card("hSD02-005", (Japanese, 229), 4, db),
                        card("hSD02-006", (Japanese, 230), 4, db),
                        card("hSD02-007", (Japanese, 231), 2, db),
                        card("hSD02-008", (Japanese, 232), 2, db),
                        card("hSD02-009", (Japanese, 233), 2, db),
                        card("hSD02-010", (Japanese, 234), 2, db),
                        card("hSD02-011", (Japanese, 235), 2, db),
                        card("hSD02-012", (Japanese, 236), 4, db),
                        card("hSD02-013", (Japanese, 237), 2, db),
                        card("hSD02-014", (Japanese, 238), 2, db),
                        card("hBP01-104", (Japanese, 145), 1, db),
                        card("hBP01-108", (Japanese, 149), 1, db),
                        card("hSD01-016", (Japanese, 16), 4, db),
                        card("hSD01-017", (Japanese, 17), 2, db),
                        card("hSD01-018", (Japanese, 18), 4, db),
                    ],
                    cheer_deck: vec![card("hY03-001", (Japanese, 170), 20, db)],
                },
            },
            // hSD03 - スタートデッキ 青 猫又おかゆ
            DeckEntry {
                deck_id: "hSD03-001".into(),
                display: "hSD03 - Start Deck (Blue) Nekomata Okayu".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Blue) Nekomata Okayu".into()),
                    oshi: Some(card("hSD03-001", (Japanese, 239), 1, db)),
                    main_deck: vec![
                        card("hSD03-002", (Japanese, 240), 6, db),
                        card("hSD03-003", (Japanese, 241), 4, db),
                        card("hSD03-004", (Japanese, 242), 2, db),
                        card("hSD03-005", (Japanese, 243), 4, db),
                        card("hSD03-006", (Japanese, 244), 4, db),
                        card("hSD03-007", (Japanese, 245), 2, db),
                        card("hSD03-008", (Japanese, 246), 2, db),
                        card("hSD03-009", (Japanese, 247), 2, db),
                        card("hSD03-010", (Japanese, 248), 2, db),
                        card("hSD03-011", (Japanese, 249), 2, db),
                        card("hSD03-012", (Japanese, 250), 4, db),
                        card("hSD03-013", (Japanese, 251), 2, db),
                        card("hSD03-014", (Japanese, 252), 2, db),
                        card("hBP01-105", (Japanese, 146), 2, db),
                        card("hBP01-108", (Japanese, 149), 2, db),
                        card("hSD01-016", (Japanese, 16), 4, db),
                        card("hSD01-017", (Japanese, 17), 2, db),
                        card("hSD01-019", (Japanese, 19), 2, db),
                    ],
                    cheer_deck: vec![card("hY04-001", (Japanese, 171), 20, db)],
                },
            },
            // hSD04 - スタートデッキ 紫 癒月ちょこ
            DeckEntry {
                deck_id: "hSD04-001".into(),
                display: "hSD04 - Start Deck (Purple) Yuzuki Choco".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Purple) Yuzuki Choco".into()),
                    oshi: Some(card("hSD04-001", (Japanese, 253), 1, db)),
                    main_deck: vec![
                        card("hSD04-002", (Japanese, 254), 6, db),
                        card("hSD04-003", (Japanese, 255), 4, db),
                        card("hSD04-004", (Japanese, 256), 2, db),
                        card("hSD04-005", (Japanese, 257), 4, db),
                        card("hSD04-006", (Japanese, 258), 4, db),
                        card("hSD04-007", (Japanese, 259), 2, db),
                        card("hSD04-008", (Japanese, 260), 2, db),
                        card("hSD04-009", (Japanese, 261), 2, db),
                        card("hSD04-010", (Japanese, 262), 2, db),
                        card("hSD04-011", (Japanese, 263), 2, db),
                        card("hSD04-012", (Japanese, 264), 4, db),
                        card("hSD04-013", (Japanese, 265), 2, db),
                        card("hSD04-014", (Japanese, 266), 2, db),
                        card("hBP01-104", (Japanese, 145), 2, db),
                        card("hBP01-106", (Japanese, 147), 2, db),
                        card("hSD01-016", (Japanese, 16), 4, db),
                        card("hSD01-017", (Japanese, 17), 2, db),
                        card("hSD01-019", (Japanese, 19), 2, db),
                    ],
                    cheer_deck: vec![card("hY05-001", (Japanese, 267), 20, db)],
                },
            },
            // hSD05 - スタートデッキ 白 轟はじめ
            DeckEntry {
                deck_id: "hSD05-001".into(),
                display: "hSD05 - Start Deck (White) Todoroki Hajime".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (White) Todoroki Hajime".into()),
                    oshi: Some(card("hSD05-001", (Japanese, 517), 1, db)),
                    main_deck: vec![
                        card("hSD05-002", (Japanese, 518), 6, db),
                        card("hSD05-003", (Japanese, 519), 4, db),
                        card("hSD05-004", (Japanese, 520), 2, db),
                        card("hSD05-005", (Japanese, 521), 4, db),
                        card("hSD05-006", (Japanese, 522), 4, db),
                        card("hSD05-007", (Japanese, 523), 2, db),
                        card("hSD05-008", (Japanese, 524), 2, db),
                        card("hSD05-009", (Japanese, 525), 2, db),
                        card("hSD05-010", (Japanese, 526), 2, db),
                        card("hSD05-011", (Japanese, 527), 2, db),
                        card("hSD05-012", (Japanese, 528), 2, db),
                        card("hSD05-013", (Japanese, 529), 2, db),
                        card("hSD05-014", (Japanese, 531), 2, db),
                        card("hSD01-016", (Japanese, 16), 4, db),
                        card("hSD01-017", (Japanese, 17), 2, db),
                        card("hBP01-104", (Japanese, 145), 2, db),
                        card("hBP01-108", (Japanese, 149), 2, db),
                        card("hPR-002", (Japanese, 530), 4, db),
                    ],
                    cheer_deck: vec![card("hY01-001", (Japanese, 168), 20, db)],
                },
            },
            // hSD06 - スタートデッキ 緑 風真いろは
            DeckEntry {
                deck_id: "hSD06-001".into(),
                display: "hSD06 - Start Deck (Green) Kazama Iroha".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Green) Kazama Iroha".into()),
                    oshi: Some(card("hSD06-001", (Japanese, 532), 1, db)),
                    main_deck: vec![
                        card("hBP01-048", (Japanese, 533), 6, db),
                        card("hSD06-002", (Japanese, 534), 4, db),
                        card("hSD06-003", (Japanese, 535), 2, db),
                        card("hSD06-004", (Japanese, 536), 4, db),
                        card("hSD06-005", (Japanese, 537), 4, db),
                        card("hBP01-050", (Japanese, 538), 2, db),
                        card("hSD06-006", (Japanese, 539), 2, db),
                        card("hSD06-007", (Japanese, 540), 2, db),
                        card("hSD06-008", (Japanese, 541), 2, db),
                        card("hSD06-009", (Japanese, 542), 2, db),
                        card("hSD06-010", (Japanese, 543), 2, db),
                        card("hSD06-011", (Japanese, 544), 2, db),
                        card("hSD06-012", (Japanese, 545), 2, db),
                        card("hSD01-016", (Japanese, 16), 4, db),
                        card("hSD01-017", (Japanese, 17), 2, db),
                        card("hBP01-104", (Japanese, 145), 2, db),
                        card("hBP02-076", (Japanese, 343), 2, db),
                        card("hBP02-080", (Japanese, 347), 4, db),
                    ],
                    cheer_deck: vec![card("hY02-001", (Japanese, 169), 20, db)],
                },
            },
            // hSD07 - スタートデッキ 黄 不知火フレア
            DeckEntry {
                deck_id: "hSD07-001".into(),
                display: "hSD07 - Start Deck (Yellow) Shiranui Flare".into(),
                oshi_options: None,
                deck: CommonDeck {
                    name: Some("Start Deck (Yellow) Shiranui Flare".into()),
                    oshi: Some(card("hSD07-001", (Japanese, 546), 1, db)),
                    main_deck: vec![
                        card("hSD07-002", (Japanese, 547), 6, db),
                        card("hSD07-003", (Japanese, 548), 4, db),
                        card("hSD07-004", (Japanese, 549), 2, db),
                        card("hSD07-005", (Japanese, 550), 4, db),
                        card("hSD07-006", (Japanese, 551), 4, db),
                        card("hSD07-007", (Japanese, 552), 2, db),
                        card("hSD07-008", (Japanese, 553), 2, db),
                        card("hSD07-009", (Japanese, 554), 2, db),
                        card("hSD07-010", (Japanese, 555), 2, db),
                        card("hSD07-011", (Japanese, 556), 2, db),
                        card("hSD07-012", (Japanese, 557), 2, db),
                        card("hSD07-013", (Japanese, 558), 2, db),
                        card("hSD07-014", (Japanese, 559), 4, db),
                        card("hSD07-015", (Japanese, 560), 2, db),
                        card("hSD01-016", (Japanese, 16), 4, db),
                        card("hSD01-017", (Japanese, 17), 2, db),
                        card("hSD01-018", (Japanese, 18), 2, db),
                        card("hBP01-104", (Japanese, 145), 2, db),
                    ],
                    cheer_deck: vec![card("hY06-001", (Japanese, 561), 20, db)],
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
                                if let Some(oshi) = oshi.card_info(&db.read()) {
                                    option { value: "{idx}",
                                        "{oshi.card_number} - {oshi.name.english.as_deref().or(oshi.name.japanese.as_deref()).unwrap_or(\"Unknown\")}"
                                    }
                                } else {
                                    option { value: "{idx}", "{oshi.card_number}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
