use dioxus::prelude::*;
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use serde::Serialize;

use crate::{
    CardLanguage, CardType,
    sources::{CommonCard, CommonDeck, price_check::PriceCache},
    tracker::{EventType, track_event},
};

static CARD_DETAILS_LANG: GlobalSignal<CardLanguage> = Signal::global(|| CardLanguage::English);

#[derive(Serialize)]
struct EventData {
    action: String,
}

#[component]
pub fn CardDetailsTitle(card: CommonCard, db: Signal<CardsDatabase>) -> Element {
    let card = use_signal(|| card);
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
    card: CommonCard,
    card_type: CardType,
    db: Signal<CardsDatabase>,
    common_deck: Option<Signal<CommonDeck>>,
    prices: Option<Signal<PriceCache>>,
) -> Element {
    let error_img_path: &str = match card_type {
        CardType::Oshi | CardType::Cheer => "cheer-back.webp",
        CardType::Main => "card-back.webp",
    };
    let error_img_path =
        format!("https://qrimpuff.github.io/hocg-fan-sim-assets/img/{error_img_path}");

    let card = use_signal(|| card);
    let lang = CARD_DETAILS_LANG.signal();
    let mut big_card = use_signal(|| false);

    let _error_img_path = error_img_path.clone();
    let img_path = use_memo({
        move || {
            card.read()
                .image_path(&db.read(), *lang.read(), false)
                .unwrap_or_else(|| _error_img_path.clone())
        }
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

    // TODO not only yuyutei
    let price_url = use_memo(move || {
        let db = db.read();
        let card = card.read().card_illustration(&db)?;

        card.yuyutei_sell_url.clone()
    });
    let price = use_memo(move || {
        if let Some(prices) = prices {
            card.read()
                .price(&db.read(), &prices.read())
                .map(|p| p.to_formatted_string(&Locale::en))
                .unwrap_or("?".into())
        } else {
            "?".into()
        }
    });

    // verify card amount
    let total_amount = use_memo(move || {
        if let Some(common_deck) = common_deck {
            common_deck
                .read()
                .all_cards()
                .filter(|c| c.card_number == card.read().card_number)
                .map(|c| c.amount)
                .sum::<u32>()
        } else {
            0
        }
    });
    let max_amount = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return 0;
        };

        card.max_amount
    });
    let warning_amount = use_memo(move || *total_amount.read() > *max_amount.read());
    let warning_class = use_memo(move || {
        if *warning_amount.read() {
            "is-warning"
        } else {
            "is-dark"
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

        // TODO add left and right arrows to navigate between cards
        // what if a card carousel for alternative illustrations?
        // or an horizontal scrollable list of cards, under the main picture?
        // TODO add/remove card buttons (max amount)
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
