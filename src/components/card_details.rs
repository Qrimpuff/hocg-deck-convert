use std::vec;

use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
use itertools::Itertools;
use serde::Serialize;

use crate::{
    CARDS_DB, COMMON_DECK, CardLanguage, CardType, EDIT_DECK, SHOW_CARD_DETAILS,
    components::{card::Card, modal_popup::ModelPopup},
    sources::{CommonCard, CommonDeck},
    tracker::{EventType, track_event},
};

static CARD_DETAILS_LANG: GlobalSignal<CardLanguage> = Signal::global(|| CardLanguage::English);

#[derive(Serialize)]
struct EventData {
    action: String,
}

#[component]
pub fn CardDetailsPopup(card: CommonCard, card_type: CardType) -> Element {
    let mut popup_card = use_signal(|| card.clone());

    // update the card when the popup is opened
    let _ = use_effect(move || {
        if *SHOW_CARD_DETAILS.read() {
            popup_card.write().clone_from(&card);
        }
    });

    rsx! {
        ModelPopup {
            show_popup: SHOW_CARD_DETAILS.signal(),
            title: rsx! {
                CardDetailsTitle { card: popup_card, db: CARDS_DB.signal() }
            },
            content: rsx! {
                CardDetailsContent {
                    card: popup_card,
                    card_type,
                    db: CARDS_DB.signal(),
                    common_deck: COMMON_DECK.signal(),
                    is_edit: EDIT_DECK.signal(),
                }
            },
        }
    }
}

#[component]
pub fn CardDetailsTitle(card: Signal<CommonCard>, db: Signal<CardsDatabase>) -> Element {
    let mut lang = CARD_DETAILS_LANG.signal();

    let title = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "<Unknown card>".to_string();
        };

        if *lang.read() == CardLanguage::Japanese {
            card.name.japanese.clone()
        } else {
            card.name
                .english
                .clone()
                .unwrap_or("<No english translation>".to_string())
        }
    });
    let subtitle = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_illustration(&db) else {
            return card.read().card_number.to_string();
        };

        format!("{} ({})", card.card_number, card.rarity)
    });

    rsx! {
        div { class: "is-flex is-justify-content-space-between",
            h4 {
                div { class: "subtitle", "{subtitle}" }
                div { class: "title", "{title}" }
            }
            div { class: "is-flex is-align-items-center mr-3",
                div { class: "buttons has-addons is-flex-shrink-0",
                    button {
                        r#type: "button",
                        class: "button is-small",
                        class: if *lang.read() == CardLanguage::English { "is-link is-selected" },
                        onclick: move |_| {
                            *lang.write() = CardLanguage::English;
                            track_event(
                                EventType::EditDeck,
                                EventData {
                                    action: "Details language EN".into(),
                                },
                            );
                        },
                        "EN"
                    }
                    button {
                        r#type: "button",
                        class: "button is-small",
                        class: if *lang.read() == CardLanguage::Japanese { "is-link is-selected" },
                        onclick: move |_| {
                            *lang.write() = CardLanguage::Japanese;
                            track_event(
                                EventType::EditDeck,
                                EventData {
                                    action: "Details language JP".into(),
                                },
                            );
                        },
                        "JP"
                    }
                }
            }
        }
    }
}

#[component]
pub fn CardDetailsContent(
    card: Signal<CommonCard>,
    card_type: CardType,
    db: Signal<CardsDatabase>,
    common_deck: Option<Signal<CommonDeck>>,
    is_edit: Signal<bool>,
) -> Element {
    let error_img_path: &str = match card_type {
        CardType::Oshi | CardType::Cheer => "cheer-back.webp",
        CardType::Main => "card-back.webp",
    };
    let error_img_path =
        format!("https://qrimpuff.github.io/hocg-fan-sim-assets/img/{error_img_path}");

    let lang = CARD_DETAILS_LANG.signal();
    let mut big_card = use_signal(|| false);

    let _error_img_path = error_img_path.clone();
    let img_path = use_memo({
        move || {
            card.read()
                .image_path(&db.read(), *lang.read(), false, true)
                .unwrap_or_else(|| _error_img_path.clone())
        }
    });

    let _db = db.read();
    let card_lang = use_signal(|| CardLanguage::Japanese);
    let alt_cards = card
        .read()
        .alt_cards(&_db)
        .into_iter()
        .map(move |mut cc| {
            if let Some(common_deck) = common_deck {
                let common_deck = common_deck.read();
                cc.amount = cc
                    .manage_id
                    .as_ref()
                    .and_then(|id| common_deck.find_card(*id))
                    .map(|c| c.amount)
                    .unwrap_or(0);
            };
            let id = format!("card-details-alt_{}", cc.manage_id.as_ref().unwrap_or(&0));
            rsx! {
                div { id,
                    Card {
                        card: cc,
                        card_type: CardType::Main,
                        card_lang,
                        is_preview: false,
                        db,
                        common_deck,
                        is_edit,
                        card_detail: Some(card),
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    // scroll currently selected into view
    let _ = use_effect(move || {
        let id = format!(
            "card-details-alt_{}",
            card.read().manage_id.as_ref().unwrap_or(&0)
        );
        document().eval(format!("
            var target = document.getElementById('{id}');
            target.parentNode.scrollLeft = target.offsetLeft - target.parentNode.offsetLeft - target.parentNode.offsetWidth / 2 + target.offsetWidth / 2;
        "));
    });

    let card_type = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "<Unknown>".to_string();
        };

        let mut card_type = if *lang.read() == CardLanguage::Japanese {
            match card.card_type {
                hocg::CardType::OshiHoloMember => "推しホロメン",
                hocg::CardType::HoloMember => match card.bloom_level {
                    Some(hocg::BloomLevel::Debut) => "Debut ホロメン",
                    Some(hocg::BloomLevel::First) => "1st ホロメン",
                    Some(hocg::BloomLevel::Second) => "2nd ホロメン",
                    Some(hocg::BloomLevel::Spot) => "Spot ホロメン",
                    None => "ホロメン",
                },
                hocg::CardType::Support(support_type) => match support_type {
                    hocg::SupportType::Staff => "サポート・スタッフ",
                    hocg::SupportType::Item => "サポート・アイテム",
                    hocg::SupportType::Event => "サポート・イベント",
                    hocg::SupportType::Tool => "サポート・ツール",
                    hocg::SupportType::Mascot => "サポート・マスコット",
                    hocg::SupportType::Fan => "サポート・ファン",
                },
                hocg::CardType::Cheer => "エール",
                hocg::CardType::Other => "Other",
            }
        } else {
            match card.card_type {
                hocg::CardType::OshiHoloMember => "Oshi Holo Member",
                hocg::CardType::HoloMember => match card.bloom_level {
                    Some(hocg::BloomLevel::Debut) => "Debut Holo Member",
                    Some(hocg::BloomLevel::First) => "1st Holo Member",
                    Some(hocg::BloomLevel::Second) => "2nd Holo Member",
                    Some(hocg::BloomLevel::Spot) => "Spot Holo Member",
                    None => "Holo Member",
                },
                hocg::CardType::Support(support_type) => match support_type {
                    hocg::SupportType::Staff => "Support - Staff",
                    hocg::SupportType::Item => "Support - Item",
                    hocg::SupportType::Event => "Support - Event",
                    hocg::SupportType::Tool => "Support - Tool",
                    hocg::SupportType::Mascot => "Support - Mascot",
                    hocg::SupportType::Fan => "Support - Fan",
                },
                hocg::CardType::Cheer => "Cheer",
                hocg::CardType::Other => "Other",
            }
        }
        .to_string();
        if card.buzz {
            if *lang.read() == CardLanguage::Japanese {
                card_type.push_str("・Buzz");
            } else {
                card_type.push_str(" - Buzz");
            }
        }
        if card.limited {
            if *lang.read() == CardLanguage::Japanese {
                card_type.push_str("・LIMITED");
            } else {
                card_type.push_str(" - Limited");
            }
        }

        let colors = card
            .colors
            .iter()
            .filter_map(|c| {
                if *lang.read() == CardLanguage::Japanese {
                    match c {
                        hocg::Color::White => Some("白"),
                        hocg::Color::Green => Some("緑"),
                        hocg::Color::Red => Some("赤"),
                        hocg::Color::Blue => Some("青"),
                        hocg::Color::Purple => Some("紫"),
                        hocg::Color::Yellow => Some("黄"),
                        hocg::Color::Colorless => None,
                    }
                } else {
                    match c {
                        hocg::Color::White => Some("White"),
                        hocg::Color::Green => Some("Green"),
                        hocg::Color::Red => Some("Red"),
                        hocg::Color::Blue => Some("Blue"),
                        hocg::Color::Purple => Some("Purple"),
                        hocg::Color::Yellow => Some("Yellow"),
                        hocg::Color::Colorless => None,
                    }
                }
            })
            .join("/");

        if colors.is_empty() {
            card_type
        } else if *lang.read() == CardLanguage::Japanese {
            format!("{}・{}", colors, card_type)
        } else {
            format!("{} - {}", colors, card_type)
        }
    });

    let life_hp = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "";
        };

        if card.life > 0 {
            "Life: "
        } else if card.hp > 0 {
            "HP: "
        } else {
            ""
        }
    });
    let life_hp_amount = use_memo(move || {
        let db = db.read();
        let card = card.read().card_info(&db)?;

        if card.life > 0 {
            Some(card.life)
        } else if card.hp > 0 {
            Some(card.hp)
        } else {
            None
        }
    });

    let card_text = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "<Unknown card>".to_string();
        };

        if *lang.read() == CardLanguage::Japanese {
            card.text.japanese.clone()
        } else {
            card.text
                .english
                .clone()
                .unwrap_or("<No english translation>".to_string())
        }
    });

    let tags = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return vec![];
        };

        card.tags
            .iter()
            .map(|t| {
                if *lang.read() == CardLanguage::Japanese {
                    t.japanese.clone()
                } else {
                    t.english
                        .clone()
                        .unwrap_or("<No english translation>".to_string())
                }
            })
            .collect::<Vec<_>>()
    });

    let baton_pass = use_memo(move || {
        let db = db.read();
        let card = card.read().card_info(&db)?;

        if card.card_type == hocg::CardType::HoloMember {
            Some(if *lang.read() == CardLanguage::Japanese {
                format!("バトンタッチ: {}", card.baton_pass.len())
            } else {
                format!("Baton Pass: {}", card.baton_pass.len())
            })
        } else {
            None
        }
    });

    rsx! {
        div { class: "block is-flex is-justify-content-center",
            a {
                href: "#",
                role: "button",
                class: "card-img-details",
                class: if *big_card.read() { "big-card" },
                style: if *big_card.read() { "cursor: zoom-out;" } else { "cursor: zoom-in;" },
                onclick: move |evt| {
                    evt.prevent_default();
                    big_card.toggle();
                    track_event(
                        EventType::EditDeck,
                        EventData {
                            action: "Card zoom".into(),
                        },
                    );
                },
                figure { class: "image",
                    img {
                        border_radius: "3.7%",
                        src: "{img_path}",
                        "onerror": "this.src='{error_img_path}'",
                    }
                }
            }
        }

        // an horizontal scrollable list of alternative illustrations
        div {
            class: "block is-flex is-flex-wrap-nowrap pb-2",
            style: "overflow-x: auto; justify-content: safe center;",
            for illust in alt_cards {
                {illust}
            }
        }

        div { class: "is-flex is-justify-content-space-between",
            div { class: "title is-5", "{card_type}" }
            div { class: "is-flex-shrink-0",
                if let Some(life_hp_amount) = *life_hp_amount.read() {
                    span { class: "subtitle is-5", "{life_hp}" }
                    span { class: "title is-5", "{life_hp_amount}" }
                }
            }
        }
        div { class: "block", style: "white-space: pre-line;", "{card_text}" }
        div { class: "block",
            for tag in &*tags.read() {
                span { class: "tag", "{tag}" }
                " "
            }
            if let Some(baton_pass) = baton_pass.read().as_ref() {
                div { "{baton_pass}" }
            }
        }
    }
}
