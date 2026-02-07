use dioxus::{document::document, logger::tracing::debug, prelude::*};
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase, SupportType};
use itertools::Itertools;
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;
use wana_kana::utils::katakana_to_hiragana;

use crate::{
    ALL_CARDS_SORTED, CardLanguage, CardType,
    components::card::Card,
    sources::{CommonCard, CommonDeck, ImageOptions},
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
    DebutUnlimited,
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

#[derive(PartialEq, Eq, Clone)]
enum FilterRarity {
    All,
    NoAlternateArt,
    Rarity(String),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FilterRelease {
    All,
    Japanese,
    English,
    Unreleased,
}

#[derive(Clone, Copy)]
struct Filters<'a> {
    text: &'a str,
    card_type: &'a FilterCardType,
    color: &'a FilterColor,
    bloom_level: &'a FilterBloomLevel,
    tag: &'a FilterTag,
    rarity: &'a FilterRarity,
    release: &'a FilterRelease,
}

pub fn prepare_text_cache(card: &hocg::Card) -> String {
    let mut text_cache = String::new();

    // Basic info
    text_cache.push_str(&card.card_number);
    text_cache.push('\n');
    text_cache.push_str(card.name.japanese.as_deref().unwrap_or_default());
    text_cache.push('\n');
    text_cache.push_str(card.name.english.as_deref().unwrap_or_default());
    text_cache.push('\n');
    text_cache.push_str(&format!("{:?}", card.card_type));
    text_cache.push('\n');
    text_cache.push_str(&format!("{:?}", card.colors));
    text_cache.push('\n');
    text_cache.push_str(&card.life.to_string());
    text_cache.push('\n');
    text_cache.push_str(&card.hp.to_string());
    text_cache.push('\n');
    text_cache.push_str(&format!("{:?}", card.bloom_level));
    text_cache.push('\n');
    text_cache.push_str(if card.buzz { "buzz" } else { "" });
    text_cache.push('\n');
    text_cache.push_str(if card.limited { "limited" } else { "" });
    text_cache.push('\n');
    // Oshi skills
    for skill in &card.oshi_skills {
        text_cache.push_str(&format!(
            "[ホロパワー：-{}]\n",
            String::from(skill.holo_power).to_uppercase()
        ));
        text_cache.push_str(&format!(
            "[holo Power: -{}]\n",
            String::from(skill.holo_power).to_uppercase()
        ));
        text_cache.push_str(skill.name.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(skill.name.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(skill.ability_text.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(skill.ability_text.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
    }
    // Arts
    for art in &card.arts {
        text_cache.push_str(art.name.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(art.name.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
        if let Some(ability_text) = &art.ability_text {
            text_cache.push_str(ability_text.japanese.as_deref().unwrap_or_default());
            text_cache.push('\n');
            text_cache.push_str(ability_text.english.as_deref().unwrap_or_default());
            text_cache.push('\n');
        }
    }
    // Keywords
    for keyword in &card.keywords {
        text_cache.push_str(&format!("{:?} effect", keyword.effect));
        text_cache.push('\n');
        text_cache.push_str(keyword.name.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(keyword.name.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(keyword.ability_text.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(keyword.ability_text.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
    }
    // Ability text
    text_cache.push_str(card.ability_text.japanese.as_deref().unwrap_or_default());
    text_cache.push('\n');
    text_cache.push_str(card.ability_text.english.as_deref().unwrap_or_default());
    text_cache.push('\n');
    // Extra
    if let Some(extra) = &card.extra {
        text_cache.push_str(extra.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(extra.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
    }
    // Tags
    for tag in &card.tags {
        text_cache.push_str(tag.japanese.as_deref().unwrap_or_default());
        text_cache.push('\n');
        text_cache.push_str(tag.english.as_deref().unwrap_or_default());
        text_cache.push('\n');
    }

    // normalize text
    normalize_filter_text(&text_cache)
}

fn normalize_filter_text(text: &str) -> String {
    katakana_to_hiragana(&text.nfkc().collect::<String>().trim().to_lowercase())
}

// return a list of cards that match the filters
fn filter_cards(
    all_cards: &[(hocg::Card, String)],
    filters: Filters,
) -> Vec<(hocg::CardIllustration, usize)> {
    // normalize the filter to hiragana, lowercase and remove extra spaces
    let filter = normalize_filter_text(filters.text);

    // group by quotes, for exact matching
    // if there's an unclosed trailing quote, treat the last split as non-exact (outside quotes)
    let parts = filter.split('"').collect_vec();
    let mut exact_parts = parts
        .into_iter()
        .zip([false, true].into_iter().cycle())
        .collect_vec();
    if let Some((_, exact)) = exact_parts.last_mut() {
        *exact = false;
    }
    let filter = exact_parts
        .into_iter()
        .flat_map(|(part, is_exact)| {
            if is_exact {
                vec![part]
            } else {
                part.split_whitespace().collect_vec()
            }
        })
        .filter(|f| !f.is_empty())
        .collect_vec();

    fn check_filter(filter: &str, text: &str) -> bool {
        if filter.is_empty() {
            return true; // if the filter is empty, we match everything
        }

        normalize_filter_text(text).contains(filter)
    }

    all_cards
        .iter()
        .flat_map(|(card, cache)| {
            card.illustrations
                .iter()
                .enumerate()
                .map(move |(n, i)| ((card, cache), i, n))
        })
        // filter by text
        .filter(|((_, cache), illust, _)| {
            // check that all words matches
            filter.iter().all(|filter| {
                let mut found = false;
                found |= cache.contains(filter);
                // Illustrator
                found |= illust
                    .illustrator
                    .iter()
                    .any(|illustrator| check_filter(filter, illustrator));
                found
            })
        })
        // filter by card type
        .filter(
            |((card, _), _, _)| match (filters.card_type, card.card_type) {
                (FilterCardType::All, _) => true,
                (FilterCardType::OshiHoloMember, hocg::CardType::OshiHoloMember) => true,
                (FilterCardType::HoloMember, hocg::CardType::HoloMember) => true,
                (FilterCardType::Support, hocg::CardType::Support(_)) => true,
                (FilterCardType::SupportStaff, hocg::CardType::Support(SupportType::Staff)) => true,
                (FilterCardType::SupportItem, hocg::CardType::Support(SupportType::Item)) => true,
                (FilterCardType::SupportEvent, hocg::CardType::Support(SupportType::Event)) => true,
                (FilterCardType::SupportTool, hocg::CardType::Support(SupportType::Tool)) => true,
                (FilterCardType::SupportMascot, hocg::CardType::Support(SupportType::Mascot)) => {
                    true
                }
                (FilterCardType::SupportFan, hocg::CardType::Support(SupportType::Fan)) => true,
                (FilterCardType::SupportLimited, hocg::CardType::Support(_)) => card.limited,
                (FilterCardType::Cheer, hocg::CardType::Cheer) => true,
                _ => false,
            },
        )
        // filter by color
        .filter(|((card, _), _, _)| match filters.color {
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
        .filter(
            |((card, _), _, _)| match (filters.bloom_level, card.bloom_level) {
                (FilterBloomLevel::All, _) => true,
                (FilterBloomLevel::Debut, Some(hocg::BloomLevel::Debut)) => true,
                (FilterBloomLevel::DebutUnlimited, Some(hocg::BloomLevel::Debut)) => {
                    card.extra.as_ref().is_some_and(|extra| {
                        extra.japanese.as_deref()
                            == Some("このホロメンはデッキに何枚でも入れられる")
                            || extra.english.as_deref()
                                == Some("You may include any number of this holomem in the deck")
                    })
                }
                (FilterBloomLevel::First, Some(hocg::BloomLevel::First)) => true,
                (FilterBloomLevel::FirstBuzz, Some(hocg::BloomLevel::First)) => card.buzz,
                (FilterBloomLevel::Second, Some(hocg::BloomLevel::Second)) => true,
                (FilterBloomLevel::Spot, Some(hocg::BloomLevel::Spot)) => true,
                _ => false,
            },
        )
        // filter by tag
        .filter(|((card, _), _, _)| match filters.tag {
            FilterTag::All => true,
            FilterTag::Tag(tag) => {
                card.tags.iter().any(|t| {
                    t.japanese
                        .as_ref()
                        .map(|t| t.to_lowercase() == tag.to_lowercase())
                        .unwrap_or_default()
                }) || card.tags.iter().any(|t| {
                    t.english
                        .as_ref()
                        .map(|t| t.to_lowercase() == tag.to_lowercase())
                        .unwrap_or_default()
                })
            }
        })
        // filter by rarity
        .filter(|((card, _), illust, n)| match filters.rarity {
            FilterRarity::All => true,
            FilterRarity::NoAlternateArt => {
                // if the card is a cheer card, use the card number with 001
                if card.card_type == hocg::CardType::Cheer
                    && let Some(num) = card
                        .card_number
                        .split_once('-')
                        .map(|(_, num)| num)
                        .and_then(|num| num.parse::<usize>().ok())
                {
                    num == 1
                } else {
                    // otherwise, the first card printed
                    *n == 0
                }
            }
            FilterRarity::Rarity(rarity) => illust.rarity.eq_ignore_ascii_case(rarity),
        })
        // filter by release
        .filter(|(_, illust, _)| match filters.release {
            FilterRelease::All => true,
            FilterRelease::Japanese => illust.manage_id.japanese.is_some(),
            FilterRelease::English => illust.manage_id.english.is_some(),
            FilterRelease::Unreleased => !illust.manage_id.has_value(),
        })
        // remove duplicate looking cards in search
        .unique_by(|(_, i, n)| {
            (
                &i.card_number,
                i.delta_art_index.unwrap_or(u32::MAX - *n as u32),
            )
        })
        .map(|(_, illustration, n)| (illustration.clone(), n))
        .collect::<Vec<_>>()

    // TODO sort by relevance
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
        #[serde(skip_serializing_if = "Option::is_none")]
        filter_type: Option<String>,
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
    let all_rarities = use_memo(move || {
        let db = db.read();
        let mut rarities = db
            .values()
            .flat_map(|card| card.illustrations.iter())
            .map(|i| i.rarity.clone())
            .unique()
            .collect::<Vec<_>>();
        // TODO not sure about sort
        rarities.sort();
        rarities
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
    let mut filter_rarity = use_signal(|| FilterRarity::All);
    let mut disable_filter_rarity = use_signal(|| false);
    let mut filter_release = use_signal(|| FilterRelease::All);
    let mut disable_filter_release = use_signal(|| false);

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
                    filter_type: None,
                },
            );
        }
    };

    let filtered_cards = use_memo(move || {
        let all_cards = ALL_CARDS_SORTED.read();
        let filter_text = cards_filter.read();
        let filter_type = filter_card_type.read();
        let filter_color = filter_color.read();
        let filter_bloom_level = filter_bloom_level.read();
        let filter_tag = filter_tag.read();
        let filter_rarity = filter_rarity.read();
        let filter_release = filter_release.read();
        filter_cards(
            &all_cards,
            Filters {
                text: &filter_text,
                card_type: &filter_type,
                color: &filter_color,
                bloom_level: &filter_bloom_level,
                tag: &filter_tag,
                rarity: &filter_rarity,
                release: &filter_release,
            },
        )
    });

    let mut card_lang = use_signal(|| CardLanguage::English);
    let _ = use_effect(move || {
        debug!("update_cards called");
        let _common_deck = common_deck.read();
        let _filtered_cards = filtered_cards.read();
        *max_card_amount.write() = _filtered_cards.len();
        *cards.write() = _filtered_cards
            .iter()
            // limit the number of cards shown
            .take(*card_amount.read())
            .map(move |(card, idx)| {
                let mut card = CommonCard::from_card_illustration(card, 0, &db.read());
                card.amount = _common_deck.card_amount(&card.card_number, Some(*idx));
                rsx! {
                    Card {
                        card,
                        card_type: CardType::Main,
                        card_lang,
                        is_preview: false,
                        image_options: ImageOptions::card_search(),
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
                div { class: "grid",
                    // Card type
                    div { class: "field cell",
                        label { "for": "card_type", class: "label", "Card type" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "card_type",
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
                                                    filter_type: Some("Card type".into()),
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
                    div { class: "field cell",
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
                                                    filter_type: Some("Color".into()),
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
                    div { class: "field cell",
                        label { "for": "card_bloom_level", class: "label", "Bloom level" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "card_bloom_level",
                                    disabled: *disable_filter_bloom_level.read(),
                                    oninput: move |ev| {
                                        *filter_bloom_level.write() = match ev.value().as_str() {
                                            "debut" => FilterBloomLevel::Debut,
                                            "debut_unlimited" => FilterBloomLevel::DebutUnlimited,
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
                                                    filter_type: Some("Bloom level".into()),
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
                                        value: "debut_unlimited",
                                        selected: *filter_bloom_level.read() == FilterBloomLevel::DebutUnlimited,
                                        "Debut - Unlimited"
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
                    div { class: "field cell",
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
                                                    filter_type: Some("Tag".into()),
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
                    // Rarity
                    div { class: "field cell",
                        label { "for": "card_rarity", class: "label", "Rarity" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "card_rarity",
                                    disabled: *disable_filter_rarity.read(),
                                    oninput: move |ev| {
                                        *filter_rarity.write() = match ev.value().as_str() {
                                            "all" => FilterRarity::All,
                                            "no_alt" => FilterRarity::NoAlternateArt,
                                            _ => FilterRarity::Rarity(ev.value()),
                                        };
                                        scroll_to_top();
                                        if *filter_rarity.read() != FilterRarity::All {
                                            track_event(
                                                EventType::EditDeck,
                                                EventData {
                                                    action: "Advanced filtering".into(),
                                                    filter_type: Some("Rarity".into()),
                                                },
                                            );
                                        }
                                    },
                                    option {
                                        value: "all",
                                        selected: *filter_rarity.read() == FilterRarity::All,
                                        "All"
                                    }
                                    option {
                                        value: "no_alt",
                                        selected: *filter_rarity.read() == FilterRarity::NoAlternateArt,
                                        "No Alternate Art"
                                    }
                                    for rarity in all_rarities.iter() {
                                        option {
                                            value: rarity.clone(),
                                            selected: *filter_rarity.read() == FilterRarity::Rarity(rarity.clone()),
                                            "{rarity}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Release
                    div { class: "field cell",
                        label { "for": "card_release", class: "label", "Release" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "card_release",
                                    disabled: *disable_filter_release.read(),
                                    oninput: move |ev| {
                                        *filter_release.write() = match ev.value().as_str() {
                                            "jp" => FilterRelease::Japanese,
                                            "en" => FilterRelease::English,
                                            "unreleased" => FilterRelease::Unreleased,
                                            _ => FilterRelease::All,
                                        };
                                        *card_lang.write() = match *filter_release.read() {
                                            FilterRelease::Japanese => CardLanguage::Japanese,
                                            FilterRelease::English => CardLanguage::English,
                                            FilterRelease::Unreleased => CardLanguage::Japanese,
                                            _ => CardLanguage::English,
                                        };
                                        scroll_to_top();
                                        if *filter_release.read() != FilterRelease::All {
                                            track_event(
                                                EventType::EditDeck,
                                                EventData {
                                                    action: "Advanced filtering".into(),
                                                    filter_type: Some("Release".into()),
                                                },
                                            );
                                        }
                                    },
                                    option {
                                        value: "all",
                                        selected: *filter_release.read() == FilterRelease::All,
                                        "All"
                                    }
                                    option {
                                        value: "jp",
                                        selected: *filter_release.read() == FilterRelease::Japanese,
                                        "Japanese"
                                    }
                                    option {
                                        value: "en",
                                        selected: *filter_release.read() == FilterRelease::English,
                                        "English"
                                    }
                                    option {
                                        value: "unreleased",
                                        selected: *filter_release.read() == FilterRelease::Unreleased,
                                        "Unreleased"
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
                                *filter_rarity.write() = FilterRarity::All;
                                *disable_filter_rarity.write() = false;
                                *filter_release.write() = FilterRelease::All;
                                *disable_filter_release.write() = false;
                                *cards_filter.write() = String::new();
                                *card_amount.write() = CARD_INCREMENT;
                                scroll_to_top();
                                track_event(
                                    EventType::EditDeck,
                                    EventData {
                                        action: "Reset filters".into(),
                                        filter_type: None,
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
        if *max_card_amount.read() > 0 {
            p { class: "has-text-grey", style: "font-size: 0.9rem",
                if *max_card_amount.read() == 1 {
                    "Found 1 card"
                } else {
                    "Found {max_card_amount} cards"
                }
            }
        }
        div {
            id: "card_search_cards",
            class: "block is-flex is-flex-wrap-wrap is-justify-content-center",
            style: "max-height: 65vh; overflow: scroll;",
            // automatically load more cards when scrolled to the bottom
            onscroll: move |_| {
                if is_scrolled_to_bottom() {
                    *loading.write() = true;
                    *card_amount.write() += CARD_INCREMENT;
                    track_event(
                        EventType::EditDeck,
                        EventData {
                            action: "Load more cards".into(),
                            filter_type: None,
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
                                        filter_type: None,
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
