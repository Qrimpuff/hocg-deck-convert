use dioxus::{document::document, logger::tracing::debug, prelude::*};
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase, SupportType};
use itertools::Itertools;
use serde::Serialize;
use wana_kana::utils::katakana_to_hiragana;

use crate::{
    CardLanguage, CardType,
    components::card::Card,
    sources::{CommonCard, CommonDeck},
    tracker::{EventType, track_event},
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum FilterCardType {
    All,
    OshiHoloMember,
    HoloMember,
    Support,
    SupportStaff,
    SupportItem,
    SupportEvent,
    SupportTool,
    SupportMascot,
    SupportFan,
    SupportLimited,
    Cheer,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FilterColor {
    All,
    White,
    Green,
    Red,
    Blue,
    Purple,
    Yellow,
    Colorless,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FilterBloomLevel {
    All,
    Debut,
    First,
    FirstBuzz,
    Second,
    Spot,
}

#[derive(PartialEq, Eq, Clone)]
enum FilterTag {
    All,
    Tag(String),
}

// return a list of cards that match the filters
fn filter_cards<'a>(
    filter: &str,
    card_type: &FilterCardType,
    color: &FilterColor,
    bloom_level: &FilterBloomLevel,
    tag: &FilterTag,
    db: &'a CardsDatabase,
) -> Vec<&'a hocg::CardIllustration> {
    // normalize the filter to hiragana, lowercase and remove extra spaces
    let filter = katakana_to_hiragana(&filter.trim().to_lowercase());
    let filter = filter.split_whitespace().collect_vec();

    let mut cards = db
        .values()
        // filter by text
        .filter(|card| {
            // check that all words matches
            filter.iter().all(|filter| {
                card.card_number.to_lowercase().contains(filter)
                    || katakana_to_hiragana(&card.name.japanese.to_lowercase()).contains(filter)
                    || card
                        .name
                        .english
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(filter))
                        .unwrap_or_default()
                    || format!("{:?}", card.card_type)
                        .to_lowercase()
                        .contains(filter)
                    || format!("{:?}", card.colors).to_lowercase().contains(filter)
                    || card.life.to_string().contains(filter)
                    || card.hp.to_string().contains(filter)
                    || format!("{:?}", card.bloom_level)
                        .to_lowercase()
                        .contains(filter)
                    || card
                        .buzz
                        .then_some("buzz")
                        .unwrap_or_default()
                        .contains(filter)
                    || card
                        .limited
                        .then_some("limited")
                        .unwrap_or_default()
                        .contains(filter)
                    || katakana_to_hiragana(&card.text.japanese.to_lowercase()).contains(filter)
                    || card
                        .text
                        .english
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(filter))
                        .unwrap_or_default()
                    || card.tags.iter().any(|tag| {
                        katakana_to_hiragana(&tag.japanese.to_lowercase()).contains(filter)
                    })
                    || card.tags.iter().any(|tag| {
                        tag.english
                            .as_ref()
                            .map(|t| t.to_lowercase().contains(filter))
                            .unwrap_or_default()
                    })
            })
        })
        // filter by card type
        .filter(|card| match (card_type, card.card_type) {
            (FilterCardType::All, _) => true,
            (FilterCardType::OshiHoloMember, hocg::CardType::OshiHoloMember) => true,
            (FilterCardType::HoloMember, hocg::CardType::HoloMember) => true,
            (FilterCardType::Support, hocg::CardType::Support(_)) => true,
            (FilterCardType::SupportStaff, hocg::CardType::Support(SupportType::Staff)) => true,
            (FilterCardType::SupportItem, hocg::CardType::Support(SupportType::Item)) => true,
            (FilterCardType::SupportEvent, hocg::CardType::Support(SupportType::Event)) => true,
            (FilterCardType::SupportTool, hocg::CardType::Support(SupportType::Tool)) => true,
            (FilterCardType::SupportMascot, hocg::CardType::Support(SupportType::Mascot)) => true,
            (FilterCardType::SupportFan, hocg::CardType::Support(SupportType::Fan)) => true,
            (FilterCardType::SupportLimited, hocg::CardType::Support(_)) => card.limited,
            (FilterCardType::Cheer, hocg::CardType::Cheer) => true,
            _ => false,
        })
        // filter by color
        .filter(|card| match color {
            FilterColor::All => true,
            FilterColor::White => card.colors.contains(&hocg::Color::White),
            FilterColor::Green => card.colors.contains(&hocg::Color::Green),
            FilterColor::Red => card.colors.contains(&hocg::Color::Red),
            FilterColor::Blue => card.colors.contains(&hocg::Color::Blue),
            FilterColor::Purple => card.colors.contains(&hocg::Color::Purple),
            FilterColor::Yellow => card.colors.contains(&hocg::Color::Yellow),
            FilterColor::Colorless => card.colors.contains(&hocg::Color::Colorless),
        })
        // filter by bloom level
        .filter(|card| match (bloom_level, card.bloom_level) {
            (FilterBloomLevel::All, _) => true,
            (FilterBloomLevel::Debut, Some(hocg::BloomLevel::Debut)) => true,
            (FilterBloomLevel::First, Some(hocg::BloomLevel::First)) => true,
            (FilterBloomLevel::FirstBuzz, Some(hocg::BloomLevel::First)) => card.buzz,
            (FilterBloomLevel::Second, Some(hocg::BloomLevel::Second)) => true,
            (FilterBloomLevel::Spot, Some(hocg::BloomLevel::Spot)) => true,
            _ => false,
        })
        // filter by tag
        .filter(|card| match tag {
            FilterTag::All => true,
            FilterTag::Tag(tag) => {
                card.tags
                    .iter()
                    .any(|t| t.japanese.to_lowercase() == tag.to_lowercase())
                    || card.tags.iter().any(|t| {
                        t.english
                            .as_ref()
                            .map(|t| t.to_lowercase() == tag.to_lowercase())
                            .unwrap_or_default()
                    })
            }
        })
        .collect_vec();

    // TODO sort by relevance
    cards.sort();

    cards
        .into_iter()
        .flat_map(|card| &card.illustrations)
        .collect()
}

fn scroll_to_top() {
    document().eval("document.getElementById('card_search_cards').scrollTop = 0;".into());
}

fn is_scrolled_to_bottom() -> bool {
    let cards = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("#card_search_cards")
        .unwrap()
        .unwrap();
    let scroll_top = cards.scroll_top();
    let scroll_height = cards.scroll_height();
    let client_height = cards.client_height();

    scroll_height - scroll_top - client_height < 2
}

#[component]
pub fn CardSearch(
    db: Signal<CardsDatabase>,
    common_deck: Signal<CommonDeck>,
    is_edit: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
    }

    let mut cards = use_signal(Vec::new);
    let mut cards_filter = use_signal(String::new);
    const CARD_INCREMENT: usize = 120; // split even for 6, 5, 4, etc. columns
    let mut card_amount = use_signal(|| CARD_INCREMENT);
    let mut max_card_amount = use_signal(|| 0);
    let all_tags = use_memo(move || {
        let db = db.read();
        let mut tags = db
            .values()
            .flat_map(|card| card.tags.iter())
            .filter_map(|c| c.english.as_ref())
            .unique()
            .cloned()
            .collect::<Vec<_>>();
        tags.sort();
        tags
    });
    let mut loading = use_signal(|| false);

    let mut show_filters = use_signal(|| false);
    let mut filter_card_type = use_signal(|| FilterCardType::All);
    let mut disable_filter_card_type = use_signal(|| false);
    let mut filter_color = use_signal(|| FilterColor::All);
    let mut disable_filter_color = use_signal(|| false);
    let mut filter_bloom_level = use_signal(|| FilterBloomLevel::All);
    let mut disable_filter_bloom_level = use_signal(|| false);
    let mut filter_tag = use_signal(|| FilterTag::All);
    let mut disable_filter_tag = use_signal(|| false);

    let update_filter = move |event: Event<FormData>| {
        let filter = event.value();
        *cards_filter.write() = filter.clone();
        *card_amount.write() = CARD_INCREMENT;
        // scroll to top, after updating the filter, to show the first cards
        scroll_to_top();

        if !filter.trim().is_empty() {
            track_event(
                EventType::EditDeck,
                EventData {
                    action: "Card search".into(),
                },
            );
        }
    };

    let card_lang = use_signal(|| CardLanguage::Japanese);
    let _ = use_effect(move || {
        debug!("update_cards called");
        let filter_text = cards_filter.read();
        let filter_type = filter_card_type.read();
        let filter_color = filter_color.read();
        let filter_bloom_level = filter_bloom_level.read();
        let filter_tag = filter_tag.read();
        let _db = db.read();
        let _common_deck = common_deck.read();
        let filtered_cards = filter_cards(
            &filter_text,
            &filter_type,
            &filter_color,
            &filter_bloom_level,
            &filter_tag,
            &_db,
        );
        *max_card_amount.write() = filtered_cards.len();
        *cards.write() = filtered_cards
            .into_iter()
            // limit the number of cards shown
            .take(*card_amount.read())
            .map(move |card| {
                rsx! {
                    Card {
                        card: CommonCard {
                            manage_id: card.manage_id,
                            card_number: card.card_number.clone(),
                            amount: card
                                .manage_id
                                .and_then(|id| _common_deck.find_card(id))
                                .map(|c| c.amount)
                                .unwrap_or(0),
                        },
                        card_type: CardType::Main,
                        card_lang,
                        is_preview: false,
                        db,
                        common_deck,
                        is_edit,
                    }
                }
            })
            .collect::<Vec<_>>();
        *loading.write() = false;
    });

    rsx! {
        // Card search
        div { class: "field",
            label { "for": "card_search", class: "label", "Card search" }
            div { class: "control",
                input {
                    id: "card_search",
                    class: "input",
                    r#type: "text",
                    oninput: update_filter,
                    value: "{cards_filter}",
                    maxlength: 100,
                    placeholder: "Search for a card... (e.g. Tokino Sora, hSD01-001, etc.)",
                }
            }
        }
        // Advanced filtering
        div { class: " block",
            a {
                href: "#",
                role: "button",
                onclick: move |evt| {
                    evt.prevent_default();
                    let show = *show_filters.read();
                    *show_filters.write() = !show;
                },
                span { class: "icon",
                    i {
                        class: "fa-solid",
                        class: if *show_filters.read() { "fa-chevron-down" } else { "fa-chevron-right" },
                    }
                }
                "Advanced filtering"
            }
        }
        if *show_filters.read() {
            div { class: "block",
                // Card type
                div { class: "field",
                    label { "for": "card_type", class: "label", "Card type" }
                    div { class: "control",
                        div { class: "select",
                            select {
                                id: "card_language",
                                disabled: *disable_filter_card_type.read(),
                                oninput: move |ev| {
                                    *filter_card_type.write() = match ev.value().as_str() {
                                        "oshi" => FilterCardType::OshiHoloMember,
                                        "member" => FilterCardType::HoloMember,
                                        "support" => FilterCardType::Support,
                                        "support_staff" => FilterCardType::SupportStaff,
                                        "support_item" => FilterCardType::SupportItem,
                                        "support_event" => FilterCardType::SupportEvent,
                                        "support_tool" => FilterCardType::SupportTool,
                                        "support_mascot" => FilterCardType::SupportMascot,
                                        "support_fan" => FilterCardType::SupportFan,
                                        "support_limited" => FilterCardType::SupportLimited,
                                        "cheer" => FilterCardType::Cheer,
                                        _ => FilterCardType::All,
                                    };
                                    scroll_to_top();
                                    match *filter_card_type.read() {
                                        FilterCardType::All => {
                                            *disable_filter_color.write() = false;
                                            *disable_filter_bloom_level.write() = false;
                                        }
                                        FilterCardType::OshiHoloMember => {
                                            *disable_filter_color.write() = false;
                                            *disable_filter_bloom_level.write() = true;
                                            *filter_bloom_level.write() = FilterBloomLevel::All;
                                        }
                                        FilterCardType::HoloMember => {
                                            *disable_filter_color.write() = false;
                                            *disable_filter_bloom_level.write() = false;
                                        }
                                        FilterCardType::Cheer => {
                                            *disable_filter_color.write() = false;
                                            *disable_filter_bloom_level.write() = true;
                                            *filter_bloom_level.write() = FilterBloomLevel::All;
                                        }
                                        _ => {
                                            *disable_filter_color.write() = true;
                                            *filter_color.write() = FilterColor::All;
                                            *disable_filter_bloom_level.write() = true;
                                            *filter_bloom_level.write() = FilterBloomLevel::All;
                                        }
                                    }
                                    if *filter_card_type.read() != FilterCardType::All {
                                        track_event(
                                            EventType::EditDeck,
                                            EventData {
                                                action: "Advanced filtering".into(),
                                            },
                                        );
                                    }
                                },
                                option {
                                    value: "all",
                                    selected: *filter_card_type.read() == FilterCardType::All,
                                    "All"
                                }
                                option {
                                    value: "oshi",
                                    selected: *filter_card_type.read() == FilterCardType::OshiHoloMember,
                                    "Oshi Holo Member"
                                }
                                option {
                                    value: "member",
                                    selected: *filter_card_type.read() == FilterCardType::HoloMember,
                                    "Holo Member"
                                }
                                option {
                                    value: "support",
                                    selected: *filter_card_type.read() == FilterCardType::Support,
                                    "Support"
                                }
                                option {
                                    value: "support_staff",
                                    selected: *filter_card_type.read() == FilterCardType::SupportStaff,
                                    "Support - Staff"
                                }
                                option {
                                    value: "support_item",
                                    selected: *filter_card_type.read() == FilterCardType::SupportItem,
                                    "Support - Item"
                                }
                                option {
                                    value: "support_event",
                                    selected: *filter_card_type.read() == FilterCardType::SupportEvent,
                                    "Support - Event"
                                }
                                option {
                                    value: "support_tool",
                                    selected: *filter_card_type.read() == FilterCardType::SupportTool,
                                    "Support - Tool"
                                }
                                option {
                                    value: "support_mascot",
                                    selected: *filter_card_type.read() == FilterCardType::SupportMascot,
                                    "Support - Mascot"
                                }
                                option {
                                    value: "support_fan",
                                    selected: *filter_card_type.read() == FilterCardType::SupportFan,
                                    "Support - Fan"
                                }
                                option {
                                    value: "support_limited",
                                    selected: *filter_card_type.read() == FilterCardType::SupportLimited,
                                    "Support - Limited"
                                }
                                option {
                                    value: "cheer",
                                    selected: *filter_card_type.read() == FilterCardType::Cheer,
                                    "Cheer"
                                }
                            }
                        }
                    }
                }
                // Color
                div { class: "field",
                    label { "for": "card_color", class: "label", "Color" }
                    div { class: "control",
                        div { class: "select",
                            select {
                                id: "card_color",
                                disabled: *disable_filter_color.read(),
                                oninput: move |ev| {
                                    *filter_color.write() = match ev.value().as_str() {
                                        "white" => FilterColor::White,
                                        "green" => FilterColor::Green,
                                        "red" => FilterColor::Red,
                                        "blue" => FilterColor::Blue,
                                        "purple" => FilterColor::Purple,
                                        "yellow" => FilterColor::Yellow,
                                        "colorless" => FilterColor::Colorless,
                                        _ => FilterColor::All,
                                    };
                                    scroll_to_top();
                                    if *filter_color.read() != FilterColor::All {
                                        track_event(
                                            EventType::EditDeck,
                                            EventData {
                                                action: "Advanced filtering".into(),
                                            },
                                        );
                                    }
                                },
                                option {
                                    value: "all",
                                    selected: *filter_color.read() == FilterColor::All,
                                    "All"
                                }
                                option {
                                    value: "white",
                                    selected: *filter_color.read() == FilterColor::White,
                                    "White"
                                }
                                option {
                                    value: "green",
                                    selected: *filter_color.read() == FilterColor::Green,
                                    "Green"
                                }
                                option {
                                    value: "red",
                                    selected: *filter_color.read() == FilterColor::Red,
                                    "Red"
                                }
                                option {
                                    value: "blue",
                                    selected: *filter_color.read() == FilterColor::Blue,
                                    "Blue"
                                }
                                option {
                                    value: "purple",
                                    selected: *filter_color.read() == FilterColor::Purple,
                                    "Purple"
                                }
                                option {
                                    value: "yellow",
                                    selected: *filter_color.read() == FilterColor::Yellow,
                                    "Yellow"
                                }
                                option {
                                    value: "colorless",
                                    selected: *filter_color.read() == FilterColor::Colorless,
                                    "Colorless"
                                }
                            }
                        }
                    }
                }
                // Bloom level
                div { class: "field",
                    label { "for": "card_bloom_level", class: "label", "Bloom level" }
                    div { class: "control",
                        div { class: "select",
                            select {
                                id: "card_bloom_level",
                                disabled: *disable_filter_bloom_level.read(),
                                oninput: move |ev| {
                                    *filter_bloom_level.write() = match ev.value().as_str() {
                                        "debut" => FilterBloomLevel::Debut,
                                        "first" => FilterBloomLevel::First,
                                        "first_buzz" => FilterBloomLevel::FirstBuzz,
                                        "second" => FilterBloomLevel::Second,
                                        "spot" => FilterBloomLevel::Spot,
                                        _ => FilterBloomLevel::All,
                                    };
                                    scroll_to_top();
                                    if *filter_bloom_level.read() != FilterBloomLevel::All {
                                        track_event(
                                            EventType::EditDeck,
                                            EventData {
                                                action: "Advanced filtering".into(),
                                            },
                                        );
                                    }
                                },
                                option {
                                    value: "all",
                                    selected: *filter_bloom_level.read() == FilterBloomLevel::All,
                                    "All"
                                }
                                option {
                                    value: "debut",
                                    selected: *filter_bloom_level.read() == FilterBloomLevel::Debut,
                                    "Debut"
                                }
                                option {
                                    value: "first",
                                    selected: *filter_bloom_level.read() == FilterBloomLevel::First,
                                    "First"
                                }
                                option {
                                    value: "first_buzz",
                                    selected: *filter_bloom_level.read() == FilterBloomLevel::FirstBuzz,
                                    "First - Buzz"
                                }
                                option {
                                    value: "second",
                                    selected: *filter_bloom_level.read() == FilterBloomLevel::Second,
                                    "Second"
                                }
                                option {
                                    value: "spot",
                                    selected: *filter_bloom_level.read() == FilterBloomLevel::Spot,
                                    "Spot"
                                }
                            }
                        }
                    }
                }
                // Tag
                div { class: "field",
                    label { "for": "card_tag", class: "label", "Tag" }
                    div { class: "control",
                        div { class: "select",
                            select {
                                id: "card_tag",
                                disabled: *disable_filter_tag.read(),
                                oninput: move |ev| {
                                    *filter_tag.write() = match ev.value().as_str() {
                                        "all" => FilterTag::All,
                                        _ => FilterTag::Tag(ev.value()),
                                    };
                                    scroll_to_top();
                                    if *filter_tag.read() != FilterTag::All {
                                        track_event(
                                            EventType::EditDeck,
                                            EventData {
                                                action: "Advanced filtering".into(),
                                            },
                                        );
                                    }
                                },
                                option {
                                    value: "all",
                                    selected: *filter_tag.read() == FilterTag::All,
                                    "All"
                                }
                                for tag in all_tags.iter() {
                                    option {
                                        value: tag.clone(),
                                        selected: *filter_tag.read() == FilterTag::Tag(tag.clone()),
                                        "{tag}"
                                    }
                                }
                            }
                        }
                    }
                }
                // Reset
                div { class: "field",
                    div { class: "control",
                        button {
                            class: "button",
                            r#type: "button",
                            onclick: move |_| {
                                *filter_card_type.write() = FilterCardType::All;
                                *disable_filter_card_type.write() = false;
                                *filter_color.write() = FilterColor::All;
                                *disable_filter_color.write() = false;
                                *filter_bloom_level.write() = FilterBloomLevel::All;
                                *disable_filter_bloom_level.write() = false;
                                *filter_tag.write() = FilterTag::All;
                                *disable_filter_tag.write() = false;
                                *cards_filter.write() = String::new();
                                *card_amount.write() = CARD_INCREMENT;
                                scroll_to_top();
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Reset filters".into(),
                                    },
                                );
                            },
                            span { class: "icon",
                                i { class: "fa-solid fa-rotate" }
                            }
                            span { "Reset filters" }
                        }
                    }
                }
            }
        }
        div {
            id: "card_search_cards",
            class: "block is-flex is-flex-wrap-wrap is-justify-content-center",
            style: "max-height: 50vh; overflow: scroll;",
            // automatically load more cards when scrolled to the bottom
            onscroll: move |_| {
                if is_scrolled_to_bottom() {
                    *loading.write() = true;
                    *card_amount.write() += CARD_INCREMENT;
                    track_event(
                        EventType::EditDeck,
                        EventData {
                            action: "Load more cards".into(),
                        },
                    );
                }
            },

            for card in cards.read().iter() {
                {card}
            }

            // load more cards
            if *card_amount.read() < *max_card_amount.read() {
                div {
                    class: "field m-2 is-flex is-justify-content-center",
                    style: "width: 100%",
                    div { class: "control",
                        button {
                            r#type: "button",
                            class: "button",
                            class: if *loading.read() { "is-loading" },
                            onclick: move |_| {
                                *loading.write() = true;
                                *card_amount.write() += CARD_INCREMENT;
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Load more cards".into(),
                                    },
                                );
                            },
                            span { class: "icon",
                                i { class: "fa-solid fa-arrow-down" }
                            }
                            span { "Load more cards" }
                        }
                    }
                }
            }

            // no cards found
            if cards.read().is_empty() && !*loading.read() {
                div { class: "notification", style: "width: 100%", "No cards found." }
            }
        }
    }
}
