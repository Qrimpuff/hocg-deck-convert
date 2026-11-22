use std::{cmp::Reverse, ops::Not, sync::OnceLock, vec};

use dioxus::{document::document, prelude::*};
use hocg_fan_sim_assets_model::{
    self as hocg, Art, CardsDatabase, Keyword, KeywordEffect, OshiSkill,
};
use itertools::Itertools;
use regex::Regex;
use serde::Serialize;

use crate::{
    CARDS_DB, COMMON_DECK, CardLanguage, CardType, EXPORT_FORMAT, GLOBAL_RARITY, GLOBAL_RELEASE,
    PREVIEW_CARD_LANG, PRICE_SERVICE, PRODUCTS_DB,
    components::{
        card::{Card, CardImageSource, CardImg},
        card_search::{FilterField, FilterRelease, Filters, TextFilter},
        modal_popup::{ModelPopup, Popup, show_popup, update_card_details_popup},
        price_history::PriceHistory,
        tooltip::{Tooltip, TooltipPlacement},
    },
    sources::{
        CommonCard, DeckLike, DeckOrPile, DeckType, ImageOptions, csv,
        price_check::PriceCheckService,
        products::{other_lang, product_label, product_url, products_language, release_date},
    },
    tracker::{EventType, TrackEvent, track_event, track_external_url},
};

/// Products listed before "Show more".
const PRODUCTS_SHOWN: usize = 4;

pub static CARD_DETAILS_LANG: GlobalSignal<CardLanguage> = Signal::global(|| CardLanguage::English);

#[derive(Serialize)]
struct EventData {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_type: Option<String>,
}
impl TrackEvent for EventData {}

#[component]
pub fn CardDetailsPopup(
    popup_id: usize,
    card: CommonCard,
    card_type: CardType,
    nav_cards: Vec<(CommonCard, CardType)>,
    is_edit: Signal<bool>,
    show_price: Signal<bool>,
) -> Element {
    let mut popup_card = use_signal(|| card.clone());
    let mut popup_type = use_signal(|| card_type);
    let nav_cards = use_signal(|| nav_cards);
    let nav_index = use_memo(move || {
        let popup_card = popup_card.read();
        nav_cards.read().iter().position(|(c, _)| {
            c.card_number == popup_card.card_number
                && c.illustration_idx == popup_card.illustration_idx
        })
    });
    let nav_len = nav_cards.read().len();

    let mut step = move |delta: isize| {
        let Some(index) = *nav_index.read() else {
            return;
        };
        let Some(target) = index.checked_add_signed(delta) else {
            return;
        };
        if let Some((next_card, next_type)) = nav_cards.read().get(target) {
            track_event(
                EventType::EditDeck,
                EventData {
                    action: (if delta < 0 {
                        "Previous card"
                    } else {
                        "Next card"
                    })
                    .into(),
                    filter_type: None,
                },
            );
            popup_card.set(next_card.clone());
            popup_type.set(*next_type);
            update_card_details_popup(popup_id, next_card.clone(), *next_type);
        }
    };
    let nav_pos = *nav_index.read();

    let nav_controls = rsx! {
        if let Some(pos) = nav_pos && nav_len > 1 {
            div { class: "card-details-nav is-flex is-align-items-center is-flex-shrink-0",
                button {
                    r#type: "button",
                    class: "button is-small",
                    title: "Previous card",
                    "aria-label": "Previous card",
                    disabled: pos == 0,
                    onclick: move |_| step(-1),
                    span { class: "icon",
                        i { class: "fa-solid fa-chevron-left" }
                    }
                }
                span { class: "has-text-grey mx-2", "{pos + 1} / {nav_len}" }
                button {
                    r#type: "button",
                    class: "button is-small",
                    title: "Next card",
                    "aria-label": "Next card",
                    disabled: pos + 1 >= nav_len,
                    onclick: move |_| step(1),
                    span { class: "icon",
                        i { class: "fa-solid fa-chevron-right" }
                    }
                }
            }
        }
    };

    rsx! {
        ModelPopup {
            popup_id,
            title: rsx! {
                CardDetailsTitle { card: popup_card, db: CARDS_DB.signal(), nav: nav_controls }
            },
            content: rsx! {
                CardDetailsContent {
                    popup_id,
                    card: popup_card,
                    card_type: popup_type,
                    db: CARDS_DB.signal(),
                    common_deck: COMMON_DECK.signal(),
                    is_edit,
                    show_price,
                }
            },
            modal_class: Some("card-details-modal".into()),
        }
    }
}

#[component]
pub fn CardDetailsTitle(
    card: Signal<CommonCard>,
    db: Signal<CardsDatabase>,
    nav: Option<Element>,
) -> Element {
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

    // highlight cards that cause the warnings
    let is_unknown = use_memo(move || card.read().is_unknown(&db.read()));
    let is_unreleased = use_memo(move || {
        card.read()
            .is_unreleased(*PREVIEW_CARD_LANG.read(), &db.read())
    });
    let card_warnings = use_memo(move || {
        let db = db.read();
        let illust = card.read().card_illustration(&db);

        let Some(format) = *EXPORT_FORMAT.read() else {
            return vec![];
        };

        let mut warnings: Vec<String> = vec![];

        if *is_unknown.read() {
            return vec!["This card is unknown.".into()];
        }

        if matches!(
            format,
            DeckType::DeckLog | DeckType::HoloDuel | DeckType::TabletopSim
        ) && *is_unreleased.read()
        {
            if illust.is_none_or(|i| !i.manage_id.has_value()) {
                warnings.push("This card is unreleased.".into());
            } else {
                match *PREVIEW_CARD_LANG.read() {
                    CardLanguage::Japanese => {
                        warnings.push("This card is unreleased in Japanese.".into())
                    }
                    CardLanguage::English => {
                        warnings.push("This card is unreleased in English.".into())
                    }
                }
            }
        }

        if format == DeckType::ProxySheets
            && card.read().card_type(&db) != Some(CardType::Cheer)
            && card
                .read()
                .image_path(
                    &db,
                    *PREVIEW_CARD_LANG.read(),
                    ImageOptions::proxy_validation_strict(),
                )
                .is_none()
        {
            match *PREVIEW_CARD_LANG.read() {
                CardLanguage::Japanese => warnings.push("Missing Japanese proxy.".into()),
                CardLanguage::English => warnings.push("Missing English proxy.".into()),
            }
        }

        if format == DeckType::Csv {
            let note = csv::notes(&card.read(), *PREVIEW_CARD_LANG.read(), &db);
            if !note.is_empty() {
                warnings.push(format!("{note}."));
            }
        }

        if format == DeckType::PriceCheck {
            match *PRICE_SERVICE.read() {
                PriceCheckService::Yuyutei => {
                    if illust.is_none_or(|i| i.yuyutei_sell_paths.is_none()) {
                        warnings.push("No price data on Yuyu-tei.".into());
                        if *is_unreleased.read() {
                            if illust.is_none_or(|i| !i.manage_id.has_value()) {
                                warnings.push("This card is unreleased.".into());
                            } else {
                                warnings.push("This card is unreleased in Japanese.".into());
                            }
                        }
                    }
                }
                PriceCheckService::TcgPlayer => {
                    if illust.is_none_or(|i| i.tcgplayer_product_ids.is_none()) {
                        warnings.push("No price data on TCGPlayer.".into());
                        if *is_unreleased.read() {
                            if illust.is_none_or(|i| !i.manage_id.has_value()) {
                                warnings.push("This card is unreleased.".into());
                            } else {
                                warnings.push("This card is unreleased in English.".into());
                            }
                        }
                    }
                }
            }
        }

        warnings
    });

    rsx! {
        div { class: "is-flex is-justify-content-space-between card-details-header",
            h4 {
                div { class: "subtitle",
                    "{subtitle}"
                    if card_warnings.read().is_empty().not() {
                        Tooltip {
                            tooltip: card_warnings.read().join("\n"),
                            underline: false,
                            placement: TooltipPlacement::Right,
                            span { class: "icon is-small has-text-warning ml-2",
                                i { class: "fa-solid fa-triangle-exclamation" }
                            }
                        }
                    } else if !unreleased.read().1.is_empty() {
                        Tooltip {
                            tooltip: format!("This card is unreleased in {}.", unreleased.read().1),
                            underline: false,
                            placement: TooltipPlacement::Right,
                            span { class: "icon is-small has-text-info ml-2",
                                i { class: "fa-solid fa-info-circle" }
                            }
                        }
                    }
                }
                div { class: "title", "{title}" }
            }
            div { class: "card-details-controls",
                if let Some(nav) = nav {
                    {nav}
                }
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
                                    filter_type: None,
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
                                    filter_type: None,
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
    card_type: Signal<CardType>,
    db: Signal<CardsDatabase>,
    common_deck: Option<Signal<DeckOrPile>>,
    is_edit: Signal<bool>,
    show_price: Signal<bool>,
) -> Element {
    let lang = CARD_DETAILS_LANG.signal();
    let mut big_card = use_signal(|| false);
    // long product lists (reprinted cards) are collapsed until asked, for each card
    let mut show_all_products = use_signal(|| false);
    use_effect(move || {
        let _ = card.read();
        show_all_products.set(false);
    });

    let img = use_memo(move || {
        let card = card.read();
        let db = db.read();
        CardImageSource::new(
            Some(&card),
            card.image_path(&db, *lang.read(), ImageOptions::card_details()),
            *card_type.read(),
            &db,
        )
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
                        show_price,
                        card_details: Some(card),
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

        let lang = *lang.read();
        let card_type = card_type_label(card, lang);
        let colors = card
            .colors
            .iter()
            .filter_map(|c| color_label(*c, lang))
            .join("/");

        if colors.is_empty() {
            card_type
        } else if lang == CardLanguage::Japanese {
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

    // Products, in the card language (or the other one when the printing isn't released in it).
    // `(code, label, official name, date, page)`; only the code without the catalog.
    #[allow(clippy::type_complexity)]
    let products = use_memo(move || {
        let db = db.read();
        let catalog = PRODUCTS_DB.read();
        let lang = *lang.read();
        let Some(illust) = card.read().card_illustration(&db).cloned() else {
            return vec![];
        };
        illust
            .products_in(products_language(&illust, lang).into(), &catalog)
            .into_iter()
            .map(|(code, product)| match product {
                Some(product) => {
                    let label = product_label(code, product, lang);
                    (
                        code.to_string(),
                        label.show_code,
                        label.label,
                        label.title,
                        release_date(product, lang).map(String::from),
                        product_url(product, lang).map(String::from),
                    )
                }
                None => (
                    code.to_string(),
                    false,
                    code.to_string(),
                    code.to_string(),
                    None,
                    None,
                ),
            })
            .collect::<Vec<_>>()
    });

    // How the printing was distributed (promos, prizes, PR packs, set-exclusive arts), in each
    // release, the card language first. `(language tag, source)`
    let distribution = use_memo(move || {
        let db = db.read();
        let lang = *lang.read();
        let Some(illust) = card.read().card_illustration(&db).cloned() else {
            return Vec::new();
        };
        [lang, other_lang(lang)]
            .into_iter()
            .flat_map(|l| {
                let tag = match l {
                    CardLanguage::Japanese => "JP",
                    CardLanguage::English => "EN",
                };
                illust
                    .promo_sources
                    .value(l.into())
                    .iter()
                    .flatten()
                    .map(move |source| (tag, source.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
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
                    rel: "noopener",
                    onclick: |_| { track_external_url("Official hOCG site (JP)") },
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
                    rel: "noopener",
                    onclick: |_| { track_external_url("Official hOCG site (EN)") },
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
                    rel: "noopener",
                    onclick: |_| { track_external_url("ogbajoj's sheet") },
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

        // Yuyu-tei
        if let yuyutei_urls = card.yuyutei_urls()
            && !yuyutei_urls.is_empty()
        {
            let multiple = yuyutei_urls.len() > 1;
            for (i, yuyutei_url) in yuyutei_urls.iter().enumerate() {
                let number = i + 1;
                urls.push(rsx! {
                    a {
                        title: "Go to Yuyu-tei for {card.card_number}",
                        href: "{yuyutei_url}",
                        target: "_blank",
                        rel: "noopener",
                        onclick: |_| { track_external_url("Yuyu-tei") },
                        span { class: "icon",
                            i { class: "fa-solid fa-arrow-up-right-from-square" }
                        }
                        if *lang.read() == CardLanguage::Japanese {
                            "遊々亭"
                        } else {
                            "Yuyu-tei"
                        }
                        if multiple {
                            " {number}"
                        }
                    }
                });
            }
        } else {
            urls.push(rsx! {
                span { class: "is-disabled-link",
                    span { class: "icon",
                        i { class: "fa-regular fa-circle-xmark" }
                    }
                    if *lang.read() == CardLanguage::Japanese {
                        "遊々亭"
                    } else {
                        "Yuyu-tei"
                    }
                }
            });
        }

        // TCGplayer
        if let tcgplayer_urls = card.tcgplayer_urls()
            && !tcgplayer_urls.is_empty()
        {
            let multiple = tcgplayer_urls.len() > 1;
            for (i, tcgplayer_url) in tcgplayer_urls.iter().enumerate() {
                let number = i + 1;
                urls.push(rsx! {
                    a {
                        title: "Go to TCGplayer for {card.card_number}",
                        href: "{tcgplayer_url}",
                        target: "_blank",
                        rel: "noopener",
                        onclick: |_| { track_external_url("TCGplayer") },
                        span { class: "icon",
                            i { class: "fa-solid fa-arrow-up-right-from-square" }
                        }
                        if *lang.read() == CardLanguage::Japanese {
                            "TCGplayer"
                        } else {
                            "TCGplayer"
                        }
                        if multiple {
                            " {number}"
                        }
                    }
                });
            }
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
                                    filter_type: None,
                                },
                            );
                        },
                        figure { class: "image",
                            CardImg { img: img.read().clone(), eager: true }
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
                    for (tag, is_link) in tags.read().iter().cloned() {
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
                                                filter_type: Some("Tag".into()),
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

                if !products.read().is_empty() || !distribution.read().is_empty() {
                    div { class: "block",
                        if !products.read().is_empty() {
                            div {
                                if *lang.read() == CardLanguage::Japanese {
                                    "収録商品:"
                                } else {
                                    "Products:"
                                }
                            }
                            div {
                                // without any code tag, no empty code column
                                class: if products.read().iter().any(|p| p.1) { "product-list" } else { "product-list no-codes" },
                                for (code , show_code , name , title , date , url) in products
                                    .read()
                                    .iter()
                                    .take(if *show_all_products.read() { usize::MAX } else { PRODUCTS_SHOWN })
                                    .cloned()
                                {
                                    if products.read().iter().any(|p| p.1) {
                                        span {
                                            if show_code {
                                                span { class: "tag", "{code}" }
                                            }
                                        }
                                    }
                                    span {
                                        if let Some(url) = url {
                                            a {
                                                href: "{url}",
                                                target: "_blank",
                                                title: "{title} ({code})",
                                                onclick: |_| { track_external_url("Official product page") },
                                                "{name}"
                                            }
                                        } else {
                                            span { title: "{title} ({code})", "{name}" }
                                        }
                                    }
                                    span { class: "product-date", {date.unwrap_or_default()} }
                                }
                            }
                            if products.read().len() > PRODUCTS_SHOWN && !*show_all_products.read() {
                                button {
                                    r#type: "button",
                                    class: "button is-small is-text",
                                    onclick: move |_| *show_all_products.write() = true,
                                    if *lang.read() == CardLanguage::Japanese {
                                        "他 {products.read().len() - PRODUCTS_SHOWN} 件を表示"
                                    } else {
                                        "Show {products.read().len() - PRODUCTS_SHOWN} more"
                                    }
                                }
                            }
                        }
                        if !distribution.read().is_empty() {
                            div {
                                if *lang.read() == CardLanguage::Japanese {
                                    "入手方法:"
                                } else {
                                    "Sources:"
                                }
                            }
                            div { class: "product-list",
                                for (tag , source) in distribution.read().clone() {
                                    span {
                                        span { class: "tag", "{tag}" }
                                    }
                                    span { "{source}" }
                                    span {}
                                }
                            }
                        }
                    }
                }

                if let Some(urls) = urls.read().as_ref() {
                    ul {
                        for url in urls {
                            li { {url} }
                        }
                    }
                }

                if *show_price.read() {
                    PriceHistory {
                        chart_id: format!("price-history-{popup_id}"),
                        card,
                        db,
                    }
                }
            }
        }
    }
}

#[component]
pub fn OshiSkillDisplay(skill: OshiSkill, lang: Signal<CardLanguage>) -> Element {
    let oshi_skill_class = match skill.kind {
        hocg::OshiSkillKind::Normal => "oshi-skill-keyword",
        hocg::OshiSkillKind::Special => "oshi-skill-special-keyword",
        hocg::OshiSkillKind::Stage => "oshi-stage-skill-keyword",
    };
    let oshi_skill = if *lang.read() == CardLanguage::Japanese {
        match skill.kind {
            hocg::OshiSkillKind::Normal => "推しスキル",
            hocg::OshiSkillKind::Special => "SP推しスキル",
            hocg::OshiSkillKind::Stage => "推しステージスキル",
        }
    } else {
        match skill.kind {
            hocg::OshiSkillKind::Normal => "Oshi Skill",
            hocg::OshiSkillKind::Special => "SP Oshi Skill",
            hocg::OshiSkillKind::Stage => "Oshi Stage Skill",
        }
    };

    let holo_power_text = if *lang.read() == CardLanguage::Japanese {
        "ホロパワー："
    } else {
        "holo Power: "
    };
    let holo_power = format!(
        "-{}",
        skill
            .holo_power
            .map(|hp| String::from(hp).to_uppercase())
            .unwrap_or_default()
    );

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
            if skill.holo_power.is_some() {
                div { class: "subtitle is-6 mb-1",
                    "[{holo_power_text}"
                    b { "{holo_power}" }
                    "]"
                }
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
        for (cheer_img, cheer_alt) in cheers {
            span {
                class: "icon-text",
                vertical_align: "sub",
                user_select: if no_title { "none" },
                if no_title {
                    span {
                        class: "icon",
                        class: if is_small { "is-small" },
                        margin_right: "0.1rem",
                        img { src: "{cheer_img}" }
                    }
                } else {
                    Tooltip { tooltip: String::from(cheer_alt), underline: false,
                        span {
                            class: "icon",
                            class: if is_small { "is-small" },
                            margin_right: "0.1rem",
                            img { src: "{cheer_img}" }
                        }
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

/// The color of a cheer named in card text, e.g. "green" or "緑".
fn cheer_color(name: &str) -> hocg::Color {
    let s = name.to_lowercase();
    if s.contains("白") || s.contains("white") {
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
    }
}

fn parse_augmented_text(text: &str, db: &CardsDatabase) -> Vec<TextSegment> {
    static RE: OnceLock<Regex> = OnceLock::new();
    static TAGS: OnceLock<Vec<String>> = OnceLock::new();
    static COLOR_RE: OnceLock<Regex> = OnceLock::new();

    let re = RE.get_or_init(|| {
        const CARD_PATTERN: &str = r"(?P<card>(?P<c_b1>[〈<])(?P<c_name>[^〉>]+)(?P<c_b2>[〉>]))";
        const IN_CARD_PATTERN: &str = r#"(?P<in_card>(?P<i_en_1>")(?P<i_name_en>[^"]+)(?P<i_en_2>" in its card name)|(?P<i_jp_1>カード名に「)(?P<i_name_jp>[^」]+)(?P<i_jp_2>」))"#;
        const TAG_PATTERN: &str = r#"(?P<tag>#(?:\s?[^#\sを持つ"「〈<\[]+){1,5})"#;
        // a list of colors sharing one "cheers", e.g. "[green, blue, and yellow] cheers",
        // "[緑と青と黄]エール"
        const CHEER_LIST_PATTERN: &str = r"(?P<yell_list>\[?(?:白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless)(?:(?:,\s(?:and\s|or\s)?|\s(?:and|or)\s|と|か)(?:白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless))+\]?(?:\scheers?|エール))";
        // the color a cheer must not be: "cheers other than blue" is printed with an icon, but
        // "non-green cheers" with the color as text (hBP03-024), so it is matched to stay text.
        // The Japanese text writes both as "緑以外のエール", so it stays text.
        const CHEER_EXCEPT_PATTERN: &str = r"(?P<yell_except>cheers?\sother\sthan\s(?:白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless))";
        const NON_COLOR_PATTERN: &str = r"(?P<non_color>non-(?:白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless))";
        const CHEER_PATTERN: &str = r"(?P<yell>(?P<y_p1>-\d\s)?(?P<y_color>白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless)(?P<y_p2>エール|-\d|\scheers?)?)";
        const SKILL_PATTERN: &str = r#"(?P<skill>(?P<s_text>oshi skill\s|推しスキル)(?P<s_b1>["「])(?P<s_name>[^"」]+)(?P<s_b2>["」]))"#;
        const EXTRA_PATTERN: &str = r#"(?P<extra>(?P<e_text>extra\s|エクストラ)(?P<e_b1>["「])(?P<e_name>[^"」]+)(?P<e_b2>["」]))"#;
        Regex::new(
            format!(
                "(?i){CARD_PATTERN}|{IN_CARD_PATTERN}|{TAG_PATTERN}|{CHEER_LIST_PATTERN}|{CHEER_EXCEPT_PATTERN}|{NON_COLOR_PATTERN}|{CHEER_PATTERN}|{SKILL_PATTERN}|{EXTRA_PATTERN}"
            )
            .as_str(),
        )
        .unwrap()
    });

    let color_re = COLOR_RE.get_or_init(|| {
        Regex::new(r"(?i)白|緑|赤|青|紫|黄|無色|white|green|red|blue|purple|yellow|colorless")
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

        // "non-green": the color is written as text
        } else if let Some(non_color) = cap.name("non_color") {
            segments.push(TextSegment::Text(non_color.as_str().to_string()));

        // Cheer icons, one before each color of the list or exception
        } else if let Some(list) = cap.name("yell_list").or_else(|| cap.name("yell_except")) {
            let list = list.as_str();
            let mut list_end = 0;
            for color_str in color_re.find_iter(list) {
                segments.push(TextSegment::Text(
                    list[list_end..color_str.start()].to_string(),
                ));
                segments.push(TextSegment::Cheers(vec![cheer_color(color_str.as_str())]));
                segments.push(TextSegment::Text(color_str.as_str().to_string()));
                list_end = color_str.end();
            }
            segments.push(TextSegment::Text(list[list_end..].to_string()));

        // Cheer icon
        } else if let Some(color_str) = cap.name("y_color") {
            let color = cheer_color(color_str.as_str());

            let valid_cheer = cap.name("y_p1").is_some()
                || cap.name("y_p2").is_some()
                || color == hocg::Color::Colorless;

            if valid_cheer {
                if let Some(p1) = cap.name("y_p1") {
                    // could contain a negative number
                    segments.push(TextSegment::Text(p1.as_str().to_string()));
                }

                // add cheer icon while keeping original color text
                segments.push(TextSegment::Cheers(vec![color]));
                segments.push(TextSegment::Text(color_str.as_str().to_string()));

                if let Some(p2) = cap.name("y_p2") {
                    // could contain "cheer" or "エール"
                    segments.push(TextSegment::Text(p2.as_str().to_string()));
                }
            } else {
                // if it's not a valid cheer, just treat it as normal text
                segments.push(TextSegment::Text(color_str.as_str().to_string()));
            }
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
                                        filter_type: Some("Card name".into()),
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
                                        filter_type: Some("Partial card name".into()),
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
                                        filter_type: Some("Oshi skill".into()),
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
                                        filter_type: Some("Tag".into()),
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
                                        filter_type: Some("Extra".into()),
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

/// The card type, with its bloom level, buzz and limited, e.g. "Debut Holo Member - Buzz".
pub fn card_type_label(card: &hocg::Card, lang: CardLanguage) -> String {
    let mut card_type = if lang == CardLanguage::Japanese {
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
        if lang == CardLanguage::Japanese {
            card_type.push_str("・Buzz");
        } else {
            card_type.push_str(" - Buzz");
        }
    }
    if card.limited {
        if lang == CardLanguage::Japanese {
            card_type.push_str("・LIMITED");
        } else {
            card_type.push_str(" - Limited");
        }
    }
    card_type
}

/// The name of a color; colorless has none.
pub fn color_label(color: hocg::Color, lang: CardLanguage) -> Option<&'static str> {
    let japanese = lang == CardLanguage::Japanese;
    Some(match color {
        hocg::Color::White => {
            if japanese {
                "白"
            } else {
                "White"
            }
        }
        hocg::Color::Green => {
            if japanese {
                "緑"
            } else {
                "Green"
            }
        }
        hocg::Color::Red => {
            if japanese {
                "赤"
            } else {
                "Red"
            }
        }
        hocg::Color::Blue => {
            if japanese {
                "青"
            } else {
                "Blue"
            }
        }
        hocg::Color::Purple => {
            if japanese {
                "紫"
            } else {
                "Purple"
            }
        }
        hocg::Color::Yellow => {
            if japanese {
                "黄"
            } else {
                "Yellow"
            }
        }
        hocg::Color::Colorless => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use TextSegment::{Cheers, Text};

    fn parse(text: &str) -> Vec<TextSegment> {
        parse_augmented_text(text, &CardsDatabase::new())
    }

    fn icons(text: &str) -> Vec<hocg::Color> {
        parse(text)
            .into_iter()
            .filter_map(|s| match s {
                Cheers(c) => Some(c),
                _ => None,
            })
            .flatten()
            .collect()
    }

    #[test]
    fn single_cheers_get_an_icon() {
        use hocg::Color::*;
        assert_eq!(
            parse("Reveal 1 yellow cheer from your cheer deck."),
            vec![
                Text("Reveal 1 ".into()),
                Cheers(vec![Yellow]),
                Text("yellow cheer from your cheer deck.".into()),
            ]
        );
        assert_eq!(icons("[green cheer or blue cheer]"), vec![Green, Blue]);
        assert_eq!(icons("[緑エールか青エール]1枚"), vec![Green, Blue]);
    }

    #[test]
    fn color_lists_get_an_icon_per_color() {
        use hocg::Color::*;
        // hBP10-002
        assert_eq!(
            parse("Send all [green, blue, and yellow] cheers from your archive"),
            vec![
                Text("Send all [".into()),
                Cheers(vec![Green]),
                Text("green, ".into()),
                Cheers(vec![Blue]),
                Text("blue, and ".into()),
                Cheers(vec![Yellow]),
                Text("yellow] cheers from your archive".into()),
            ]
        );
        // hBP10-022, hBP09-089
        assert_eq!(
            icons("If your stage has green, blue, and yellow cheers, restore"),
            vec![Green, Blue, Yellow]
        );
        assert_eq!(
            icons("自分のステージに[緑と青と黄]エールがあるなら"),
            vec![Green, Blue, Yellow]
        );
    }

    #[test]
    fn excluded_cheers_get_an_icon() {
        use hocg::Color::*;
        // hBP06-048: the card prints the excluded color as an icon too
        assert_eq!(
            parse("If cheers other than blue are attached"),
            vec![
                Text("If cheers other than ".into()),
                Cheers(vec![Blue]),
                Text("blue are attached".into()),
            ]
        );
        // hSD12-010
        assert_eq!(
            icons("archive 1 cheer other than purple from"),
            vec![Purple]
        );
    }

    #[test]
    fn non_color_cheers_stay_text() {
        // hBP03-024, hBP02-052: printed with the color as text
        assert_eq!(
            parse("If 2 or more non-green cheers are attached"),
            vec![Text("If 2 or more non-green cheers are attached".into())]
        );
        assert_eq!(icons("If a non-purple cheer is attached"), vec![]);
        // the Japanese text can't tell these from hBP06-048's icon, so it stays text
        assert_eq!(icons("このホロメンに紫以外のエールが付いている時"), vec![]);
    }

    #[test]
    fn holomem_colors_get_no_icon() {
        // hBP09-089: the Oshi color, then a cheer without a color
        assert_eq!(
            icons(
                "If your Oshi holomem's color is green, blue or yellow, send the top card of your cheer deck"
            ),
            vec![]
        );
        assert_eq!(icons("自分の推しホロメンの色が緑か青か黄なら"), vec![]);
        assert_eq!(icons("Reveal 1 [blue holomem or purple holomem]"), vec![]);
    }
}
