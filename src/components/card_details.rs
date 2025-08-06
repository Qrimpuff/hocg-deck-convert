use std::{ops::Not, vec};

use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::{
    self as hocg, Art, CardsDatabase, Keyword, KeywordEffect, OshiSkill,
};
use itertools::Itertools;
use serde::Serialize;

use crate::{
    CARDS_DB, COMMON_DECK, CardLanguage, CardType, EDIT_DECK, SHOW_CARD_DETAILS,
    components::{card::Card, modal_popup::ModelPopup},
    sources::{CommonCard, CommonDeck, ImageOptions},
    tracker::{EventType, track_event, track_url},
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
            card.name
                .japanese
                .clone()
                .unwrap_or("<No Japanese name>".to_string())
        } else {
            card.name
                .english
                .clone()
                .unwrap_or("<No English name>".to_string())
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
    let error_img_path = format!("/hocg-deck-convert/assets/{error_img_path}");

    let lang = CARD_DETAILS_LANG.signal();
    let mut big_card = use_signal(|| false);

    let _error_img_path = error_img_path.clone();
    let img_path = use_memo({
        move || {
            card.read()
                .image_path(&db.read(), *lang.read(), ImageOptions::card_details())
                .unwrap_or_else(|| _error_img_path.clone())
        }
    });

    let _db = db.read();
    let alt_cards = card
        .read()
        .alt_cards(&_db)
        .into_iter()
        .map(move |mut cc| {
            if let Some(common_deck) = common_deck {
                let common_deck = common_deck.read();
                cc.amount = common_deck.card_amount(&cc.card_number, cc.illustration_idx);
            };
            let id = format!("card-details-alt_{}", cc.html_id());
            rsx! {
                div { id,
                    Card {
                        card: cc,
                        card_type: CardType::Main,
                        card_lang: lang,
                        is_preview: false,
                        image_options: ImageOptions::card_search(),
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
        let id = format!("card-details-alt_{}", card.read().html_id());
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
            format!("{colors}・{card_type}")
        } else {
            format!("{colors} - {card_type}")
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

    let oshi_skills = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return vec![];
        };

        // Only oshi holo member cards have oshi skills
        if !matches!(card.card_type, hocg::CardType::OshiHoloMember) {
            return vec![];
        }

        card.oshi_skills.to_vec()
    });

    let keywords = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return vec![];
        };

        // Only holo member cards have keywords
        if !matches!(card.card_type, hocg::CardType::HoloMember) {
            return vec![];
        }

        card.keywords.to_vec()
    });

    let arts = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return vec![];
        };

        // Only holo member cards have arts
        if !matches!(card.card_type, hocg::CardType::HoloMember) {
            return vec![];
        }

        card.arts.to_vec()
    });

    let card_text = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "<Unknown card>".to_string();
        };

        // Only support cards have ability text
        if !matches!(
            card.card_type,
            hocg::CardType::Support(_) | hocg::CardType::Cheer
        ) {
            return "".into();
        }

        if *lang.read() == CardLanguage::Japanese {
            card.ability_text
                .japanese
                .clone()
                .unwrap_or("<No Japanese text>".to_string())
        } else {
            card.ability_text
                .english
                .clone()
                .unwrap_or("<No English text>".to_string())
        }
    });

    let extra = use_memo(move || {
        let db = db.read();
        let card = card.read().card_info(&db)?;

        // Only holo member cards have extra text
        if !matches!(card.card_type, hocg::CardType::HoloMember) {
            return None;
        }

        card.extra.as_ref().map(|extra| {
            if *lang.read() == CardLanguage::Japanese {
                extra
                    .japanese
                    .clone()
                    .unwrap_or("<No Japanese text>".to_string())
            } else {
                extra
                    .english
                    .clone()
                    .unwrap_or("<No English text>".to_string())
            }
        })
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
                    t.japanese
                        .clone()
                        .unwrap_or("<No Japanese tag>".to_string())
                } else {
                    t.english.clone().unwrap_or("<No English tag>".to_string())
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

    let max_amount = use_memo(move || {
        let db = db.read();
        let max_amount = card.read().max_amount(*lang.read(), &db);
        let card = card.read().card_info(&db)?;

        if card.card_type != hocg::CardType::OshiHoloMember {
            Some(if *lang.read() == CardLanguage::Japanese {
                format!("最大金額: {max_amount}")
            } else {
                format!("Max amount: {max_amount}")
            })
        } else {
            None
        }
    });

    let illustrator = use_memo(move || {
        let db = db.read();
        let card = card.read().card_illustration(&db)?;

        card.illustrator
            .as_ref()
            .filter(|illustrator| !illustrator.is_empty())
            .map(|illustrator| {
                if *lang.read() == CardLanguage::Japanese {
                    format!("イラストレーター名: {illustrator}")
                } else {
                    format!("Illustrator: {illustrator}")
                }
            })
    });

    let urls = use_memo(move || {
        let db = db.read();
        let card = card.read().card_illustration(&db)?;

        let mut urls = Vec::with_capacity(2);

        // Official hOCG site (Japanese)
        if let Some(manage_id) = card.manage_id.japanese.as_ref() {
            urls.push(rsx! {
                a {
                    title: "Go to the official hOCG site (JP) for {card.card_number}",
                    href: "https://hololive-official-cardgame.com/cardlist/?id={manage_id}",
                    target: "_blank",
                    onclick: |_| { track_url("Official hOCG site (JP)") },
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-up-right-from-square" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "公式サイト"
                    } else {
                        "Official hOCG site (JP)"
                    }
                }
            });
        }

        // Official hOCG site (English)
        if let Some(manage_id) = card.manage_id.english.as_ref() {
            urls.push(rsx! {
                a {
                    title: "Go to the official hOCG site (EN) for {card.card_number}",
                    href: "https://en.hololive-official-cardgame.com/cardlist/?id={manage_id}",
                    target: "_blank",
                    onclick: |_| { track_url("Official hOCG site (EN)") },
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-up-right-from-square" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "公式サイト EN"
                    } else {
                        "Official hOCG site (EN)"
                    }
                }
            });
        }

        // Yuyutei
        if let Some(yuyutei_sell_url) = card.yuyutei_sell_url.as_ref() {
            urls.push(rsx! {
                a {
                    title: "Go to Yuyutei for {card.card_number}",
                    href: "{yuyutei_sell_url}",
                    target: "_blank",
                    onclick: |_| { track_url("Yuyutei") },
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-up-right-from-square" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "遊々亭"
                    } else {
                        "Yuyutei"
                    }
                }
            });
        }

        // TCGplayer
        if let Some(tcgplayer_url) = card.tcgplayer_url() {
            urls.push(rsx! {
                a {
                    title: "Go to TCGplayer for {card.card_number}",
                    href: "{tcgplayer_url}",
                    target: "_blank",
                    onclick: |_| { track_url("TCGplayer") },
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-up-right-from-square" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "TCGplayer"
                    } else {
                        "TCGplayer"
                    }
                }
            });
        }

        urls.is_empty().not().then_some(urls)
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
            div { class: "title is-6", "{card_type}" }
            div { class: "is-flex-shrink-0",
                if let Some(life_hp_amount) = *life_hp_amount.read() {
                    span { class: "subtitle is-5", "{life_hp}" }
                    span { class: "title is-5", "{life_hp_amount}" }
                }
            }
        }

        if !oshi_skills.read().is_empty() {
            div { class: "block",
                for skill in oshi_skills.read().iter() {
                    OshiSkillDisplay { skill: skill.clone(), lang }
                }
            }
        }

        if !keywords.read().is_empty() {
            div { class: "block",
                for keyword in keywords.read().iter() {
                    KeywordDisplay { keyword: keyword.clone(), lang }
                }
            }
        }

        if !arts.read().is_empty() {
            div { class: "block",
                for art in arts.read().iter() {
                    ArtDisplay { art: art.clone(), lang }
                }
            }
        }

        if !card_text.read().is_empty() {
            div { class: "block", style: "white-space: pre-line;", "{card_text}" }
        }

        if let Some(extra) = extra.read().as_ref() {
            div { class: "block",
                div {
                    span { class: "title is-6 pr-1 extra-keyword",
                        if *lang.read() == CardLanguage::Japanese {
                            "エクストラ"
                        } else {
                            "Extra"
                        }
                    }
                }
                div { style: "white-space: pre-line;", "{extra}" }
            }
        }

        div { class: "block",
            for tag in &*tags.read() {
                span { class: "tag", "{tag}" }
                " "
            }
            if let Some(baton_pass) = baton_pass.read().as_ref() {
                div { "{baton_pass}" }
            }
            if let Some(max_amount) = max_amount.read().as_ref() {
                div { "{max_amount}" }
            }
        }

        if let Some(illustrator) = illustrator.read().as_ref() {
            div { class: "block", "{illustrator}" }
        }

        if let Some(urls) = urls.read().as_ref() {
            ul {
                for url in urls {
                    li { {url} }
                }
            }
        }
    }
}

#[component]
pub fn OshiSkillDisplay(skill: OshiSkill, lang: Signal<CardLanguage>) -> Element {
    let oshi_skill_class = if skill.special {
        "oshi-skill-special-keyword"
    } else {
        "oshi-skill-keyword"
    };
    let oshi_skill = if *lang.read() == CardLanguage::Japanese {
        if skill.special {
            "SP推しスキル"
        } else {
            "推しスキル"
        }
    } else if skill.special {
        "SP Oshi Skill"
    } else {
        "Oshi Skill"
    };

    let holo_power_text = if *lang.read() == CardLanguage::Japanese {
        "ホロパワー："
    } else {
        "holo Power: "
    };
    let holo_power = format!("-{}", String::from(skill.holo_power).to_uppercase());

    let name = if *lang.read() == CardLanguage::Japanese {
        skill
            .name
            .japanese
            .clone()
            .unwrap_or("<No Japanese name>".to_string())
    } else {
        skill
            .name
            .english
            .clone()
            .unwrap_or("<No English name>".to_string())
    };

    let text = if *lang.read() == CardLanguage::Japanese {
        skill
            .ability_text
            .japanese
            .clone()
            .unwrap_or("<No Japanese text>".to_string())
    } else {
        skill
            .ability_text
            .english
            .clone()
            .unwrap_or("<No English text>".to_string())
    };

    rsx! {
        div { class: "block",
            div {
                span { class: "title is-6 pr-1 {oshi_skill_class}", "{oshi_skill}" }
                span { class: "title is-5 ml-2", "{name}" }
            }
            div { class: "subtitle is-6 mb-1",
                "[{holo_power_text}"
                b { "{holo_power}" }
                "]"
            }
            div { style: "white-space: pre-line;", "{text}" }
        }
    }
}

#[component]
pub fn KeywordDisplay(keyword: Keyword, lang: Signal<CardLanguage>) -> Element {
    let keyword_class = match keyword.effect {
        KeywordEffect::Collab => "collab-keyword",
        KeywordEffect::Bloom => "bloom-keyword",
        KeywordEffect::Gift => "gift-keyword",
        KeywordEffect::Other => "",
    };
    let keyword_name = if *lang.read() == CardLanguage::Japanese {
        match keyword.effect {
            KeywordEffect::Collab => "コラボエフェクト",
            KeywordEffect::Bloom => "ブルームエフェクト",
            KeywordEffect::Gift => "ギフト",
            KeywordEffect::Other => "その他",
        }
    } else {
        match keyword.effect {
            KeywordEffect::Collab => "Collab Effect",
            KeywordEffect::Bloom => "Bloom Effect",
            KeywordEffect::Gift => "Gift",
            KeywordEffect::Other => "Other",
        }
    };

    let name = if *lang.read() == CardLanguage::Japanese {
        keyword
            .name
            .japanese
            .clone()
            .unwrap_or("<No Japanese name>".to_string())
    } else {
        keyword
            .name
            .english
            .clone()
            .unwrap_or("<No English name>".to_string())
    };

    let text = if *lang.read() == CardLanguage::Japanese {
        keyword
            .ability_text
            .japanese
            .clone()
            .unwrap_or("<No Japanese text>".to_string())
    } else {
        keyword
            .ability_text
            .english
            .clone()
            .unwrap_or("<No English text>".to_string())
    };

    rsx! {
        div { class: "block",
            div {
                span { class: "title is-6 pr-1 {keyword_class}", "{keyword_name}" }
                span { class: "title is-5 ml-2", "{name}" }
            }
            div { style: "white-space: pre-line;", "{text}" }
        }
    }
}

#[component]
pub fn ArtDisplay(art: Art, lang: Signal<CardLanguage>) -> Element {
    let cheers = art.cheers.iter().map(|c| {
        let cheer_img = match c {
            hocg::Color::White => "arts_white.png",
            hocg::Color::Green => "arts_green.png",
            hocg::Color::Red => "arts_red.png",
            hocg::Color::Blue => "arts_blue.png",
            hocg::Color::Purple => "arts_purple.png",
            hocg::Color::Yellow => "arts_yellow.png",
            hocg::Color::Colorless => "arts_null.png",
        };

        let cheer_alt = if *lang.read() == CardLanguage::Japanese {
            match c {
                hocg::Color::White => "白",
                hocg::Color::Green => "緑",
                hocg::Color::Red => "赤",
                hocg::Color::Blue => "青",
                hocg::Color::Purple => "紫",
                hocg::Color::Yellow => "黄",
                hocg::Color::Colorless => "無色",
            }
        } else {
            match c {
                hocg::Color::White => "White",
                hocg::Color::Green => "Green",
                hocg::Color::Red => "Red",
                hocg::Color::Blue => "Blue",
                hocg::Color::Purple => "Purple",
                hocg::Color::Yellow => "Yellow",
                hocg::Color::Colorless => "Colorless",
            }
        };

        (format!("/hocg-deck-convert/assets/{cheer_img}"), cheer_alt)
    });

    let name = if *lang.read() == CardLanguage::Japanese {
        art.name
            .japanese
            .clone()
            .unwrap_or("<No Japanese name>".to_string())
    } else {
        art.name
            .english
            .clone()
            .unwrap_or("<No English name>".to_string())
    };

    let power_name = if *lang.read() == CardLanguage::Japanese {
        "ダメージ: "
    } else {
        "Power: "
    };
    let power = String::from(art.power);

    let advantage = art.advantage.map(|adv| {
        if *lang.read() == CardLanguage::Japanese {
            match adv.0 {
                hocg::Color::White => ("white-advantage", format!("白<b>+{}</b>", adv.1)),
                hocg::Color::Green => ("green-advantage", format!("緑<b>+{}</b>", adv.1)),
                hocg::Color::Red => ("red-advantage", format!("赤<b>+{}</b>", adv.1)),
                hocg::Color::Blue => ("blue-advantage", format!("青<b>+{}</b>", adv.1)),
                hocg::Color::Purple => ("purple-advantage", format!("紫<b>+{}</b>", adv.1)),
                hocg::Color::Yellow => ("yellow-advantage", format!("黄<b>+{}</b>", adv.1)),
                hocg::Color::Colorless => ("", format!("無色<b>+{}</b>", adv.1)),
            }
        } else {
            match adv.0 {
                hocg::Color::White => ("white-advantage", format!("<b>+{}</b> vs White", adv.1)),
                hocg::Color::Green => ("green-advantage", format!("<b>+{}</b> vs Green", adv.1)),
                hocg::Color::Red => ("red-advantage", format!("<b>+{}</b> vs Red", adv.1)),
                hocg::Color::Blue => ("blue-advantage", format!("<b>+{}</b> vs Blue", adv.1)),
                hocg::Color::Purple => ("purple-advantage", format!("<b>+{}</b> vs Purple", adv.1)),
                hocg::Color::Yellow => ("yellow-advantage", format!("<b>+{}</b> vs Yellow", adv.1)),
                hocg::Color::Colorless => ("", format!("<b>+{}</b> vs Colorless", adv.1)),
            }
        }
    });

    let text = if *lang.read() == CardLanguage::Japanese {
        art.ability_text.map(|text| {
            text.japanese
                .clone()
                .unwrap_or("<No Japanese text>".to_string())
        })
    } else {
        art.ability_text.map(|text| {
            text.english
                .clone()
                .unwrap_or("<No English text>".to_string())
        })
    };

    rsx! {
        div { class: "block",
            div { class: "is-flex",
                span { class: "is-flex-shrink-0",
                    for (cheer_img , cheer_alt) in cheers {
                        img {
                            class: "icon",
                            margin_right: "0.1rem",
                            title: "{cheer_alt}",
                            src: "{cheer_img}",
                        }
                    }
                }
                span { class: "title is-5 ml-3", "{name}" }
            }
            div { class: "subtitle is-6 mb-1",
                "{power_name}"
                b { class: "title is-6", "{power}" }
                if let Some(advantage) = advantage {
                    span {
                        class: "ml-2 {advantage.0}",
                        dangerous_inner_html: "{advantage.1}",
                    }
                }
            }
            if let Some(text) = text {
                div { style: "white-space: pre-line;", "{text}" }
            }
        }
    }
}
