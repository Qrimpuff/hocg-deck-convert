use std::{cmp::Reverse, ops::Not, sync::OnceLock, vec};

use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::{
    self as hocg, Art, CardsDatabase, Keyword, KeywordEffect, OshiSkill,
};
use itertools::Itertools;
use regex::Regex;
use serde::Serialize;

use crate::{
    CARDS_DB, COMMON_DECK, CardLanguage, CardType, EDIT_DECK, GLOBAL_RARITY, GLOBAL_RELEASE,
    components::{
        card::Card,
        card_search::{FilterField, FilterRelease, Filters, TextFilter},
        modal_popup::{ModelPopup, Popup, show_popup},
    },
    sources::{CommonCard, CommonDeck, ImageOptions},
    tracker::{EventType, track_event, track_url},
};

static CARD_DETAILS_LANG: GlobalSignal<CardLanguage> = Signal::global(|| CardLanguage::English);

#[derive(Serialize)]
struct EventData {
    action: String,
    source: Option<String>,
}

#[component]
pub fn CardDetailsPopup(popup_id: usize, card: CommonCard, card_type: CardType) -> Element {
    let popup_card = use_signal(|| card.clone());

    rsx! {
        ModelPopup {
            popup_id,
            title: rsx! {
                CardDetailsTitle { card: popup_card, db: CARDS_DB.signal() }
            },
            content: rsx! {
                CardDetailsContent {
                    popup_id,
                    card: popup_card,
                    card_type,
                    db: CARDS_DB.signal(),
                    common_deck: COMMON_DECK.signal(),
                    is_edit: EDIT_DECK.signal(),
                }
            },
            modal_class: Some("card-details-modal".into()),
        }
    }
}

#[component]
pub fn CardDetailsTitle(card: Signal<CommonCard>, db: Signal<CardsDatabase>) -> Element {
    let mut lang = CARD_DETAILS_LANG.signal();

    let title = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "- Unknown card -".to_string();
        };

        if *lang.read() == CardLanguage::Japanese {
            card.name
                .japanese
                .clone()
                .unwrap_or("- No Japanese name -".to_string())
        } else {
            card.name
                .english
                .clone()
                .unwrap_or("- No English name -".to_string())
        }
    });
    let subtitle = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_illustration(&db) else {
            return card.read().card_number.to_string();
        };

        format!("{} ({})", card.card_number, card.rarity)
    });

    let unreleased = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_illustration(&db) else {
            return (false, "");
        };

        (
            !card.manage_id.has_value(),
            if *GLOBAL_RELEASE.read() == FilterRelease::Japanese
                && card.manage_id.japanese.is_none()
            {
                "Japanese"
            } else if *GLOBAL_RELEASE.read() == FilterRelease::English
                && card.manage_id.english.is_none()
            {
                "English"
            } else {
                ""
            },
        )
    });

    rsx! {
        div { class: "is-flex is-justify-content-space-between",
            h4 {
                div { class: "subtitle",
                    "{subtitle}"
                    if unreleased.read().0 {
                        span {
                            class: "icon is-small has-text-warning ml-2",
                            title: "This card is unreleased",
                            i { class: "fa-solid fa-triangle-exclamation" }
                        }
                    } else if !unreleased.read().1.is_empty() {
                        span {
                            class: "icon is-small has-text-info ml-2",
                            title: "This card is unreleased in {unreleased.read().1}",
                            i { class: "fa-solid fa-info-circle" }
                        }
                    }
                }
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
                                    source: None,
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
                                    source: None,
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
    #[props(default)] popup_id: usize,
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
            let id = format!("card-details-alt_{}_{popup_id}", cc.html_id());
            rsx! {
                div { id,
                    Card {
                        card: cc,
                        card_type: CardType::Main,
                        card_lang: lang,
                        is_preview: false,
                        image_options: ImageOptions::card_details(),
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
        let id = format!("card-details-alt_{}_{popup_id}", card.read().html_id());
        document().eval(format!("
            var target = document.getElementById('{id}');
            target.parentNode.scrollLeft = target.offsetLeft - target.parentNode.offsetLeft - target.parentNode.offsetWidth / 2 + target.offsetWidth / 2;
        "));
    });

    let card_type = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "- Unknown -".to_string();
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
        card.read()
            .card_info(&db.read())
            .map(|card| card.oshi_skills.to_vec())
            .unwrap_or_default()
    });

    let keywords = use_memo(move || {
        card.read()
            .card_info(&db.read())
            .map(|card| card.keywords.to_vec())
            .unwrap_or_default()
    });

    let arts = use_memo(move || {
        card.read()
            .card_info(&db.read())
            .map(|card| card.arts.to_vec())
            .unwrap_or_default()
    });

    let card_text = use_memo(move || {
        let db = db.read();
        let Some(card) = card.read().card_info(&db) else {
            return "- Unknown card -".to_string();
        };

        let required =
            card.oshi_skills.is_empty() && card.keywords.is_empty() && card.arts.is_empty();
        if *lang.read() == CardLanguage::Japanese {
            card.ability_text
                .japanese
                .clone()
                .or_else(|| required.then_some("- No Japanese text -".to_string()))
                .unwrap_or_default()
        } else {
            card.ability_text
                .english
                .clone()
                .or_else(|| required.then_some("- No English text -".to_string()))
                .unwrap_or_default()
        }
    });

    let extra = use_memo(move || {
        let db = db.read();
        let card = card.read().card_info(&db)?;

        card.extra.as_ref().map(|extra| {
            if *lang.read() == CardLanguage::Japanese {
                extra
                    .japanese
                    .clone()
                    .unwrap_or("- No Japanese text -".to_string())
            } else {
                extra
                    .english
                    .clone()
                    .unwrap_or("- No English text -".to_string())
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
                        .map(|tag| (tag, true))
                        .unwrap_or(("- No Japanese tag -".to_string(), false))
                } else {
                    t.english
                        .clone()
                        .map(|tag| (tag, true))
                        .unwrap_or(("- No English tag -".to_string(), false))
                }
            })
            .collect::<Vec<_>>()
    });

    let baton_pass = use_memo(move || {
        let db = db.read();
        let card = card.read().card_info(&db)?;

        // only holo member cards have baton pass, and free baton pass is valid
        if card.card_type == hocg::CardType::HoloMember {
            Some(rsx! {
                div { class: "is-flex",
                    span { class: "mr-2",
                        if *lang.read() == CardLanguage::Japanese {
                            "バトンタッチ: "
                        } else {
                            "Baton Pass: "
                        }
                    }
                    span { class: "is-flex-shrink-0",
                        CheersDisplay {
                            cheers: card.baton_pass.clone(),
                            lang,
                            is_small: true,
                        }
                    }
                }
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
        let urls_jp = card.official_site_urls(hocg::Language::Japanese);
        for url in &urls_jp {
            urls.push(rsx! {
                a {
                    title: "Go to the official hOCG site (JP) for {card.card_number}",
                    href: "{url}",
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
        if urls_jp.is_empty() {
            urls.push(rsx! {
                span { class: "is-disabled-link",
                    span { class: "icon",
                        i { class: "fa-regular fa-circle-xmark" }
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
        let urls_en = card.official_site_urls(hocg::Language::English);
        for url in &urls_en {
            urls.push(rsx! {
                a {
                    title: "Go to the official hOCG site (EN) for {card.card_number}",
                    href: "{url}",
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
        if urls_en.is_empty() {
            urls.push(rsx! {
                span { class: "is-disabled-link",
                    span { class: "icon",
                        i { class: "fa-regular fa-circle-xmark" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "公式サイト EN"
                    } else {
                        "Official hOCG site (EN)"
                    }
                }
            });
        }

        // ogbajoj's sheet
        if let Some(ogbajoj_sheet_url) = card.ogbajoj_sheet_urls().first() {
            urls.push(rsx! {
                a {
                    title: "Go to ogbajoj's sheet for {card.card_number}",
                    href: "{ogbajoj_sheet_url}",
                    target: "_blank",
                    onclick: |_| { track_url("ogbajoj's sheet") },
                    span { class: "icon",
                        i { class: "fa-solid fa-arrow-up-right-from-square" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "ogbajojのスプレッドシート"
                    } else {
                        "ogbajoj's sheet"
                    }
                }
            });
        } else {
            urls.push(rsx! {
                span { class: "is-disabled-link",
                    span { class: "icon",
                        i { class: "fa-regular fa-circle-xmark" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "ogbajojのスプレッドシート"
                    } else {
                        "ogbajoj's sheet"
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
        } else {
            urls.push(rsx! {
                span { class: "is-disabled-link",
                    span { class: "icon",
                        i { class: "fa-regular fa-circle-xmark" }
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
        } else {
            urls.push(rsx! {
                span { class: "is-disabled-link",
                    span { class: "icon",
                        i { class: "fa-regular fa-circle-xmark" }
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

    let scroll_container_id = format!("scroll-container-{popup_id}");
    let _scroll_container_id = scroll_container_id.clone();
    let on_scroll_container_mount = move |_: MountedEvent| {
        // can't use deltaMode, it will lose pixel precision on some browsers
        document().eval(format!(
            r#"
            const element = document.getElementById('{_scroll_container_id}');
            if (element) {{
                element.addEventListener('wheel', (event) => {{
                    if (event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) {{
                        return;
                    }}
                    if (event.deltaY !== 0) {{
                        element.scrollLeft += event.deltaY;
                        event.preventDefault();
                    }}
                }}, {{ passive: false }});
            }}
            "#
        ));
    };

    rsx! {
        div { class: "columns",
            div {
                class: "column ",
                class: if *big_card.read() { "is-three-fifths" } else { "is-one-third" },
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
                                    source: None,
                                },
                            );
                        },
                        figure { class: "image",
                            img {
                                width: "400",
                                height: "560",
                                border_radius: "3.7%",
                                src: "{img_path}",
                                "onerror": "this.src='{error_img_path}'",
                            }
                        }
                    }
                }

                // an horizontal scrollable list of alternative illustrations
                div {
                    id: "{scroll_container_id}",
                    class: "block is-flex is-flex-wrap-nowrap pb-2",
                    style: "overflow-x: auto; justify-content: safe center;",
                    onmount: on_scroll_container_mount,
                    for illust in alt_cards {
                        {illust}
                    }
                }
            }

            div { class: "column",
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
                    div { class: "block", style: "white-space: pre-line;",
                        AugmentedText { text: "{card_text}", lang }
                    }
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
                        div { style: "white-space: pre-line;",
                            AugmentedText { text: "{extra}", lang }
                        }
                    }
                }

                div { class: "block",
                    for (tag , is_link) in tags.read().iter().cloned() {
                        if is_link {
                            span { class: "tag",
                                a {
                                    title: r#"Find cards tagged "{tag}""#,
                                    onclick: move |evt| {
                                        evt.prevent_default();
                                        show_popup(
                                            Popup::CardSearch(Filters {
                                                texts: vec![TextFilter::full_match(FilterField::Tag, &tag)],
                                                rarity: GLOBAL_RARITY.read().clone(),
                                                release: *GLOBAL_RELEASE.read(),
                                                ..Default::default()
                                            }),
                                        );
                                        track_event(
                                            EventType::EditDeck,
                                            EventData {
                                                action: "Card search popup".into(),
                                                source: Some("Tag".into()),
                                            },
                                        );
                                    },
                                    "{tag}"
                                }
                            }
                        } else {
                            span { class: "tag", "{tag}" }
                        }
                        " "
                    }
                    if let Some(baton_pass) = baton_pass.read().as_ref() {
                        div { {baton_pass} }
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
            .unwrap_or("- No Japanese name -".to_string())
    } else {
        skill
            .name
            .english
            .clone()
            .unwrap_or("- No English name -".to_string())
    };

    let text = if *lang.read() == CardLanguage::Japanese {
        skill
            .ability_text
            .japanese
            .clone()
            .unwrap_or("- No Japanese text -".to_string())
    } else {
        skill
            .ability_text
            .english
            .clone()
            .unwrap_or("- No English text -".to_string())
    };

    rsx! {
        div { class: "block",
            div {
                span { class: "title is-6 pr-1 {oshi_skill_class}", "{oshi_skill}" }
                span { class: "title is-5 ml-1", " {name}" }
            }
            div { class: "subtitle is-6 mb-1",
                "[{holo_power_text}"
                b { "{holo_power}" }
                "]"
            }
            div { style: "white-space: pre-line;",
                AugmentedText { text: "{text}", lang }
            }
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
            .unwrap_or("- No Japanese name -".to_string())
    } else {
        keyword
            .name
            .english
            .clone()
            .unwrap_or("- No English name -".to_string())
    };

    let text = if *lang.read() == CardLanguage::Japanese {
        keyword
            .ability_text
            .japanese
            .clone()
            .unwrap_or("- No Japanese text -".to_string())
    } else {
        keyword
            .ability_text
            .english
            .clone()
            .unwrap_or("- No English text -".to_string())
    };

    rsx! {
        div { class: "block",
            div {
                span { class: "title is-6 pr-1 {keyword_class}", "{keyword_name}" }
                span { class: "title is-5 ml-1", " {name}" }
            }
            div { style: "white-space: pre-line;",
                AugmentedText { text: "{text}", lang }
            }
        }
    }
}

#[component]
pub fn ArtDisplay(art: Art, lang: Signal<CardLanguage>) -> Element {
    let name = if *lang.read() == CardLanguage::Japanese {
        art.name
            .japanese
            .clone()
            .unwrap_or("- No Japanese name -".to_string())
    } else {
        art.name
            .english
            .clone()
            .unwrap_or("- No English name -".to_string())
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
                .unwrap_or("- No Japanese text -".to_string())
        })
    } else {
        art.ability_text.map(|text| {
            text.english
                .clone()
                .unwrap_or("- No English text -".to_string())
        })
    };

    rsx! {
        div { class: "block",
            div { class: "is-flex",
                span { class: "is-flex-shrink-0",
                    CheersDisplay { cheers: art.cheers.clone(), lang }
                }
                span { class: "title is-5 ml-3", " {name}" }
            }
            div { class: "subtitle is-6 mb-1",
                "{power_name}"
                b { class: "title is-6", "{power}" }
                if let Some(advantage) = advantage {
                    span {
                        class: "ml-1 {advantage.0}",
                        dangerous_inner_html: " {advantage.1}",
                    }
                }
            }
            if let Some(text) = text {
                div { style: "white-space: pre-line;",
                    AugmentedText { text: "{text}", lang }
                }
            }
        }
    }
}

#[component]
fn CheersDisplay(
    cheers: Vec<hocg::Color>,
    lang: Signal<CardLanguage>,
    #[props(default)] is_small: bool,
    #[props(default)] no_title: bool,
) -> Element {
    let cheers = cheers.iter().map(|c| {
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

    rsx! {
        for (cheer_img , cheer_alt) in cheers {
            span { class: "icon-text", vertical_align: "sub",
                span {
                    class: "icon",
                    class: if is_small { "is-small" } else { "" },
                    margin_right: "0.1rem",
                    img {
                        title: if !no_title { "{cheer_alt}" } else { "" },
                        src: "{cheer_img}",
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
enum TextSegment {
    Text(String),
    /// e.g. 〈Tsunomaki Watame〉
    CardName(String),
    /// e.g. with "PC" in its card name
    PartialCardName(String),
    /// e.g. SP Oshi Skill "Ollie Revives"
    OshiSkill(String),
    /// e.g. #Gen 3
    Tag(String),
    /// e.g. Extra "You may include any number of this holomem in the deck"
    Extra(String),
    /// e.g. a green cheer, 1~2 yellow cheers
    Cheers(Vec<hocg::Color>),
}

fn parse_augmented_text(text: &str, db: &CardsDatabase) -> Vec<TextSegment> {
    static RE: OnceLock<Regex> = OnceLock::new();
    static TAGS: OnceLock<Vec<String>> = OnceLock::new();

    let re = RE.get_or_init(|| {
        const CARD_PATTERN: &str = r"(?P<card>(?P<c_b1>[〈<])(?P<c_name>[^〉>]+)(?P<c_b2>[〉>]))";
        const IN_CARD_PATTERN: &str = r#"(?P<in_card>(?P<i_en_1>")(?P<i_name_en>[^"]+)(?P<i_en_2>" in its card name)|(?P<i_jp_1>カード名に「)(?P<i_name_jp>[^」]+)(?P<i_jp_2>」))"#;
        const TAG_PATTERN: &str = r"(?P<tag>#(?:\s?[^\sを持つ]+){1,5})";
        const CHEER_PATTERN: &str = r"(?P<yell>(白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless)(?P<y_text>エール|\scheers?)?)";
        const SKILL_PATTERN: &str = r#"(?P<skill>(?P<s_text>oshi skill\s|推しスキル)(?P<s_b1>["「])(?P<s_name>[^"」]+)(?P<s_b2>["」]))"#;
        const EXTRA_PATTERN: &str = r#"(?P<extra>(?P<e_text>extra\s|エクストラ)(?P<e_b1>["「])(?P<e_name>[^"」]+)(?P<e_b2>["」]))"#;
        Regex::new(
            format!(
                "(?i){CARD_PATTERN}|{IN_CARD_PATTERN}|{TAG_PATTERN}|{CHEER_PATTERN}|{SKILL_PATTERN}|{EXTRA_PATTERN}"
            )
            .as_str(),
        )
        .unwrap()
    });

    let mut segments = Vec::new();
    let mut last_end = 0;

    for cap in re.captures_iter(text) {
        let m = cap.get(0).unwrap();

        if m.start() > last_end {
            segments.push(TextSegment::Text(text[last_end..m.start()].to_string()));
        }

        // Card name
        if let Some(card_name) = cap.name("c_name") {
            segments.push(TextSegment::Text(cap["c_b1"].to_string()));
            segments.push(TextSegment::CardName(card_name.as_str().to_string()));
            segments.push(TextSegment::Text(cap["c_b2"].to_string()));

        // Partial card name
        } else if let Some(card_name) = cap.name("i_name_en").or_else(|| cap.name("i_name_jp")) {
            segments.push(TextSegment::Text(
                cap.name("i_en_1")
                    .or_else(|| cap.name("i_jp_1"))
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
            ));
            segments.push(TextSegment::PartialCardName(card_name.as_str().to_string()));
            segments.push(TextSegment::Text(
                cap.name("i_en_2")
                    .or_else(|| cap.name("i_jp_2"))
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
            ));

        // Oshi skill
        } else if let Some(skill_name) = cap.name("s_name") {
            segments.push(TextSegment::Text(cap["s_text"].to_string()));
            segments.push(TextSegment::Text(cap["s_b1"].to_string()));
            segments.push(TextSegment::OshiSkill(skill_name.as_str().to_string()));
            segments.push(TextSegment::Text(cap["s_b2"].to_string()));

        // Tag
        } else if let Some(tag_str) = cap.name("tag") {
            let all_tags = TAGS.get_or_init(|| {
                let mut tags = db
                    .values()
                    .flat_map(|card| card.tags.iter())
                    .flat_map(|t| [&t.japanese, &t.english])
                    .filter_map(|t| t.as_ref())
                    .unique()
                    .cloned()
                    .collect::<Vec<_>>();
                tags.sort_by_key(|t| Reverse(t.len()));
                tags
            });

            // verify that it's a valid tag (could be less than 3 words)
            if let Some(valid_tag) = all_tags.iter().find(|tag| {
                tag_str
                    .as_str()
                    .to_lowercase()
                    .starts_with(&tag.to_lowercase())
            }) {
                segments.push(TextSegment::Tag(valid_tag.clone()));
                segments.push(TextSegment::Text(
                    tag_str.as_str()[valid_tag.len()..].to_string(),
                ));
            } else {
                segments.push(TextSegment::Text(tag_str.as_str().to_string()));
            }

        // Extra
        } else if let Some(extra) = cap.name("e_name") {
            segments.push(TextSegment::Text(cap["e_text"].to_string()));
            segments.push(TextSegment::Text(cap["e_b1"].to_string()));
            segments.push(TextSegment::Extra(extra.as_str().to_string()));
            segments.push(TextSegment::Text(cap["e_b2"].to_string()));

        // Cheer icon
        } else if let Some(cheer_str) = cap.name("yell") {
            let s = cheer_str.as_str().to_lowercase();
            let color = if s.contains("白") || s.contains("white") {
                hocg::Color::White
            } else if s.contains("緑") || s.contains("green") {
                hocg::Color::Green
            } else if s.contains("赤") || s.contains("red") {
                hocg::Color::Red
            } else if s.contains("青") || s.contains("blue") {
                hocg::Color::Blue
            } else if s.contains("紫") || s.contains("purple") {
                hocg::Color::Purple
            } else if s.contains("黄") || s.contains("yellow") {
                hocg::Color::Yellow
            } else {
                hocg::Color::Colorless
            };

            // just add the cheer icon, it's more pleasant with the full text
            if cap.name("y_text").is_some() {
                // there is always "cheers" after a valid color
                segments.push(TextSegment::Cheers(vec![color]));
            } else if color == hocg::Color::Colorless {
                // no "cheer" text for colorless
                segments.push(TextSegment::Cheers(vec![color]));
            }
            segments.push(TextSegment::Text(cheer_str.as_str().to_string()));
        }

        last_end = m.end();
    }

    if last_end < text.len() {
        segments.push(TextSegment::Text(text[last_end..].to_string()));
    }

    // merge consecutive Text segments
    segments.into_iter().fold(Vec::new(), |mut acc, segment| {
        match segment {
            TextSegment::Text(t) => {
                if let Some(TextSegment::Text(last)) = acc.last_mut() {
                    last.push_str(&t);
                } else {
                    acc.push(TextSegment::Text(t));
                }
            }
            other => acc.push(other),
        }
        acc
    })
}

#[component]
fn AugmentedText(text: String, lang: Signal<CardLanguage>) -> Element {
    let segments = parse_augmented_text(&text, &CARDS_DB.read());

    rsx! {
        for segment in segments {
            match segment {
                TextSegment::Text(t) => {
                    rsx! { "{t}" }
                }
                TextSegment::CardName(name) => {
                    rsx! {
                        a {
                            title: r#"Find cards named "{name}""#,
                            onclick: move |evt| {
                                evt.prevent_default();
                                show_popup(
                                    Popup::CardSearch(Filters {
                                        texts: vec![TextFilter::full_match(FilterField::CardName, &name)],
                                        rarity: GLOBAL_RARITY.read().clone(),
                                        release: *GLOBAL_RELEASE.read(),
                                        ..Default::default()
                                    }),
                                );
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Card search popup".into(),
                                        source: Some("Card name".into()),
                                    },
                                );
                            },
                            "{name}"
                        }
                    }
                }
                TextSegment::PartialCardName(part) => {
                    rsx! {
                        a {
                            title: r#"Find cards with "{part}" in their name"#,
                            onclick: move |evt| {
                                evt.prevent_default();
                                show_popup(
                                    Popup::CardSearch(Filters {
                                        texts: vec![TextFilter::partial_match(FilterField::CardName, &part)],
                                        rarity: GLOBAL_RARITY.read().clone(),
                                        release: *GLOBAL_RELEASE.read(),
                                        ..Default::default()
                                    }),
                                );
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Card search popup".into(),
                                        source: Some("Partial card name".into()),
                                    },
                                );
                            },
                            "{part}"
                        }
                    }
                }
                TextSegment::OshiSkill(skill) => {
                    rsx! {
                        a {
                            title: r#"Find cards with oshi skill "{skill}""#,
                            onclick: move |evt| {
                                evt.prevent_default();
                                show_popup(
                                    Popup::CardSearch(Filters {
                                        texts: vec![TextFilter::full_match(FilterField::OshiSkill, &skill)],
                                        rarity: GLOBAL_RARITY.read().clone(),
                                        release: *GLOBAL_RELEASE.read(),
                                        ..Default::default()
                                    }),
                                );
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Card search popup".into(),
                                        source: Some("Oshi skill".into()),
                                    },
                                );
                            },
                            "{skill}"
                        }
                    }
                }
                TextSegment::Tag(tag) => {
                    rsx! {
                        a {
                            title: r#"Find cards tagged "{tag}""#,
                            onclick: move |evt| {
                                evt.prevent_default();
                                show_popup(
                                    Popup::CardSearch(Filters {
                                        texts: vec![TextFilter::full_match(FilterField::Tag, &tag)],
                                        rarity: GLOBAL_RARITY.read().clone(),
                                        release: *GLOBAL_RELEASE.read(),
                                        ..Default::default()
                                    }),
                                );
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Card search popup".into(),
                                        source: Some("Tag".into()),
                                    },
                                );
                            },
                            "{tag}"
                        }
                    }
                }
                TextSegment::Extra(extra) => {
                    rsx! {
                        a {
                            title: r#"Find cards with extra "{extra}""#,
                            onclick: move |evt| {
                                evt.prevent_default();
                                show_popup(
                                    Popup::CardSearch(Filters {
                                        texts: vec![TextFilter::full_match(FilterField::Extra, &extra)],
                                        rarity: GLOBAL_RARITY.read().clone(),
                                        release: *GLOBAL_RELEASE.read(),
                                        ..Default::default()
                                    }),
                                );
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Card search popup".into(),
                                        source: Some("Extra".into()),
                                    },
                                );
                            },
                            "{extra}"
                        }
                    }
                }
                TextSegment::Cheers(cheers) => {
                    rsx! {
                        CheersDisplay {
                            cheers: cheers.clone(),
                            is_small: true,
                            no_title: true,
                            lang,
                        }
                    }
                }
            }
        }
    }
}
