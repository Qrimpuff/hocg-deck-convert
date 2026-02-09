use dioxus::{logger::tracing::debug, prelude::*, web::WebEventExt};
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase, Localized, SupportType};
use itertools::Itertools;
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;
use wana_kana::utils::katakana_to_hiragana;

use crate::{
    ALL_CARDS_SORTED, CARDS_DB, COMMON_DECK, CardLanguage, CardType, EDIT_DECK, GLOBAL_RARITY,
    GLOBAL_RELEASE,
    components::{card::Card, modal_popup::ModelPopup},
    sources::{CommonCard, CommonDeck, ImageOptions},
    tracker::{EventType, track_event},
};

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum FilterCardType {
    #[default]
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

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum FilterColor {
    #[default]
    All,
    White,
    Green,
    Red,
    Blue,
    Purple,
    Yellow,
    Colorless,
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum FilterBloomLevel {
    #[default]
    All,
    Debut,
    DebutUnlimited,
    First,
    FirstBuzz,
    Second,
    Spot,
}

#[derive(PartialEq, Eq, Clone, Default)]
pub enum FilterTag {
    #[default]
    All,
    Tag(String),
}

impl FilterTag {
    fn to_text_filters(&self) -> Vec<TextFilter> {
        match self {
            FilterTag::All => vec![],
            FilterTag::Tag(t) => vec![TextFilter::full_match(FilterField::Tag, t)],
        }
    }
}

#[derive(PartialEq, Eq, Clone, Default)]
pub enum FilterRarity {
    #[default]
    All,
    NoAlternateArt,
    Rarity(String),
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum FilterRelease {
    #[default]
    All,
    Japanese,
    English,
    Unreleased,
}

#[derive(PartialEq, Eq, Clone, Copy, Default, Hash)]
pub enum FilterField {
    #[default]
    All,
    CardName,
    OshiSkill,
    Tag,
}

#[derive(PartialEq, Eq, Clone, Default)]
pub struct TextFilter {
    pub text: (String, bool), // has been processed
    pub field: FilterField,
    pub full_match: bool,
    pub exact_match: bool,
    pub case_sensitive: bool,
}

impl TextFilter {
    fn from_part(text: &str, exact: bool) -> Self {
        TextFilter {
            text: (text.to_string(), false),
            field: FilterField::All,
            full_match: false,
            exact_match: exact,
            case_sensitive: false,
        }
    }

    pub fn full_match(field: FilterField, text: &str) -> Self {
        TextFilter {
            text: (text.to_string(), false),
            field,
            full_match: true,
            exact_match: false,
            case_sensitive: false,
        }
    }

    fn from_text_field(text: &str) -> Vec<Self> {
        // group by quotes, for exact matching
        // if there's an unclosed trailing quote, treat the last split as non-exact (outside quotes)
        let parts = text.split('"').collect_vec();
        let mut exact_parts = parts
            .into_iter()
            .zip([false, true].into_iter().cycle())
            .collect_vec();
        if let Some((_, exact)) = exact_parts.last_mut() {
            *exact = false;
        }

        exact_parts
            .into_iter()
            .flat_map(|(part, is_exact)| {
                if is_exact {
                    vec![(part, true)]
                } else {
                    part.split_whitespace().map(|s| (s, false)).collect_vec()
                }
            })
            .filter(|f| !f.0.is_empty())
            .map(|(p, exact)| TextFilter::from_part(p, exact))
            .collect_vec()
    }

    pub fn multi_check(filters: &mut [TextFilter], text: &str) -> bool {
        let map = filters
            .iter_mut()
            .into_group_map_by(|f| (f.case_sensitive, f.exact_match));

        // only normalize once for each group of filters
        map.into_values().all(|mut filters| {
            let text = filters[0].normalize_text(text);
            filters.iter_mut().all(|f| f.check_no_normalize(&text))
        })
    }

    pub fn multi_check_localized(filters: &mut [TextFilter], text: &Localized<String>) -> bool {
        [&text.japanese, &text.english]
            .into_iter()
            .filter_map(|t| t.as_ref())
            .any(|t| Self::multi_check(filters, t))
    }

    pub fn multi_check_cache(filters: &mut [TextFilter], text_cache: &(String, String)) -> bool {
        filters.iter_mut().all(|f| {
            if f.exact_match {
                f.check_no_normalize(&text_cache.1)
            } else {
                f.check_no_normalize(&text_cache.0)
            }
        })
    }

    #[allow(unused)]
    pub fn check(&mut self, text: &str) -> bool {
        let text = self.normalize_text(text);
        self.check_no_normalize(&text)
    }

    pub fn check_no_normalize(&mut self, text: &str) -> bool {
        if self.text.0.is_empty() {
            return true; // if the filter is empty, we match everything
        }

        // if the text is already processed, we can skip normalization
        let filter = if self.text.1 {
            &self.text.0
        } else {
            self.text.0 = self.normalize_text(&self.text.0);
            self.text.1 = true;
            &self.text.0
        };

        if self.full_match {
            text == filter
        } else {
            text.contains(filter)
        }
    }

    fn normalize_text(&self, text: &str) -> String {
        // normalize the filter to hiragana, lowercase and remove extra spaces

        let text = if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        if self.exact_match {
            text.nfkc().collect::<String>()
        } else {
            katakana_to_hiragana(text.nfkc().collect::<String>().trim())
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Filters {
    pub texts: Vec<TextFilter>,
    pub card_type: FilterCardType,
    pub color: FilterColor,
    pub bloom_level: FilterBloomLevel,
    pub rarity: FilterRarity,
    pub release: FilterRelease,
}

impl Filters {
    pub fn to_filter_text(&self) -> String {
        self.texts
            .iter()
            .filter(|f| f.field == FilterField::All)
            .map(|f| {
                let mut text = f.text.0.clone();
                if f.exact_match {
                    text = format!("\"{}\"", text);
                }
                text
            })
            .join(" ")
    }

    pub fn to_filter_tag(&self) -> FilterTag {
        self.texts
            .iter()
            .find(|f| f.field == FilterField::Tag)
            .map(|f| FilterTag::Tag(f.text.0.clone()))
            .unwrap_or(FilterTag::All)
    }
}

pub fn prepare_text_cache(card: &hocg::Card) -> (String, String) {
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
    (
        TextFilter::from_part("", false).normalize_text(&text_cache),
        TextFilter::from_part("", true).normalize_text(&text_cache),
    )
}

// return a list of cards that match the filters
fn filter_cards(
    all_cards: &[(hocg::Card, (String, String))],
    filters: Filters,
) -> Vec<(hocg::CardIllustration, usize)> {
    let mut filters_texts = filters.texts.into_iter().into_group_map_by(|f| f.field);
    let mut filters_all_card = filters_texts.remove(&FilterField::All);
    let mut filters_all_illust = filters_all_card.clone();
    let mut filters_card_name = filters_texts.remove(&FilterField::CardName);
    let mut filters_oshi_skill = filters_texts.remove(&FilterField::OshiSkill);
    let mut filters_tag = filters_texts.remove(&FilterField::Tag);

    all_cards
        .iter()
        // filter by card name, if specified
        .filter(|(card, _)| {
            // extract "CardName" filter
            let Some(ref mut filters) = filters_card_name else {
                return true;
            };

            let mut found = filters.is_empty();
            found |= TextFilter::multi_check_localized(filters, &card.name);
            // include "This holomem is also regarded as ..."
            if let Some(extra) = card.extra.as_ref() {
                let mut filters = filters
                    .iter()
                    .map(|f| {
                        // need to match the name inside the extra
                        TextFilter {
                            full_match: false,
                            ..f.clone()
                        }
                    })
                    .collect_vec();
                found |= TextFilter::multi_check_localized(&mut filters, extra);
            }
            found
        })
        // filter by oshi skill, if specified
        .filter(|(card, _)| {
            // extract "OshiSkill" filter
            let Some(ref mut filters) = filters_oshi_skill else {
                return true;
            };

            let mut found = filters.is_empty();
            found |= card
                .oshi_skills
                .iter()
                .any(|skill| TextFilter::multi_check_localized(filters, &skill.name));
            found
        })
        // filter by card type
        .filter(|(card, _)| match (filters.card_type, card.card_type) {
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
        .filter(|(card, _)| match filters.color {
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
        .filter(|(card, _)| match (filters.bloom_level, card.bloom_level) {
            (FilterBloomLevel::All, _) => true,
            (FilterBloomLevel::Debut, Some(hocg::BloomLevel::Debut)) => true,
            (FilterBloomLevel::DebutUnlimited, Some(hocg::BloomLevel::Debut)) => {
                card.extra.as_ref().is_some_and(|extra| {
                    extra.japanese.as_deref() == Some("このホロメンはデッキに何枚でも入れられる")
                        || extra.english.as_deref()
                            == Some("You may include any number of this holomem in the deck")
                })
            }
            (FilterBloomLevel::First, Some(hocg::BloomLevel::First)) => true,
            (FilterBloomLevel::FirstBuzz, Some(hocg::BloomLevel::First)) => card.buzz,
            (FilterBloomLevel::Second, Some(hocg::BloomLevel::Second)) => true,
            (FilterBloomLevel::Spot, Some(hocg::BloomLevel::Spot)) => true,
            _ => false,
        })
        // filter by tag
        .filter(|(card, _)| {
            // extract "Tag" filter
            let Some(ref mut filters) = filters_tag else {
                return true;
            };

            let mut found = filters.is_empty();
            found |= card
                .tags
                .iter()
                .any(|tag| TextFilter::multi_check_localized(filters, tag));
            found
        })
        // filter by text, for cards
        .map(|(card, cache)| {
            // extract "All" filter for general search field
            let Some(ref mut filters) = filters_all_card else {
                return (card, false);
            };

            // will be fully filtered out in "filter by text, for illustrations"
            (card, TextFilter::multi_check_cache(filters, cache))
        })
        // filter illustrations
        .flat_map(|(card, found_in_cache)| {
            card.illustrations
                .iter()
                .enumerate()
                .map(move |(n, i)| (card, found_in_cache, i, n))
        })
        // filter by text, for illustrations
        .filter(|(_, found_in_cache, illust, _)| {
            // extract "All" filter for general search field
            let Some(ref mut filters) = filters_all_illust else {
                return true;
            };

            let mut found = *found_in_cache;
            // Illustrator
            found |= illust
                .illustrator
                .iter()
                .any(|illustrator| TextFilter::multi_check(filters, illustrator));
            found
        })
        // filter by rarity
        .filter(|(card, _, illust, n)| match &filters.rarity {
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
                    // otherwise, the first card printed will selected in "remove duplicate"
                    // don't show released card number in "Unreleased"
                    // the first illustration is the most likely to be released
                    filters.release != FilterRelease::Unreleased || *n == 0
                }
            }
            FilterRarity::Rarity(rarity) => illust.rarity.eq_ignore_ascii_case(rarity),
        })
        // filter by release
        .filter(|(_, _, illust, _)| match filters.release {
            FilterRelease::All => true,
            FilterRelease::Japanese => illust.manage_id.japanese.is_some(),
            FilterRelease::English => illust.manage_id.english.is_some(),
            FilterRelease::Unreleased => !illust.manage_id.has_value(),
        })
        // remove duplicate looking cards in search
        .unique_by(|(_, _, i, n)| {
            (
                &i.card_number,
                if filters.rarity == FilterRarity::NoAlternateArt {
                    0
                } else {
                    i.delta_art_index.unwrap_or(u32::MAX - *n as u32)
                },
            )
        })
        .map(|(_, _, illustration, n)| (illustration.clone(), n))
        .collect::<Vec<_>>()

    // TODO sort by relevance
}

fn scroll_to_top(container: &mut web_sys::Element) {
    container.set_scroll_top(0);
}

fn is_scrolled_to_bottom(container: &web_sys::Element) -> bool {
    let scroll_top = container.scroll_top();
    let scroll_height = container.scroll_height();
    let client_height = container.client_height();

    scroll_height - scroll_top - client_height < 2
}

#[component]
pub fn CardSearch(
    #[props(default)] popup_id: usize,
    db: Signal<CardsDatabase>,
    common_deck: Signal<CommonDeck>,
    is_edit: Signal<bool>,
    #[props(default)] default_filters: Filters,
    #[props(default = true)] show_inputs: bool,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filter_type: Option<String>,
    }

    let mut cards = use_signal(Vec::new);
    let mut cards_filter = use_signal(|| default_filters.to_filter_text());
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
    let mut card_lang = use_signal(|| match default_filters.release {
        FilterRelease::Japanese => CardLanguage::Japanese,
        FilterRelease::English => CardLanguage::English,
        FilterRelease::Unreleased => CardLanguage::Japanese,
        _ => CardLanguage::English,
    });
    let mut loading = use_signal(|| false);

    let mut container_ref = use_signal(|| None);
    let mut show_filters = use_signal(|| false);
    let mut filter_card_type = use_signal(|| default_filters.card_type);
    let mut disable_filter_card_type = use_signal(|| false);
    let mut filter_color = use_signal(|| default_filters.color);
    let mut disable_filter_color = use_signal(|| false);
    let mut filter_bloom_level = use_signal(|| default_filters.bloom_level);
    let mut disable_filter_bloom_level = use_signal(|| false);
    let mut filter_tag = use_signal(|| default_filters.to_filter_tag());
    let mut disable_filter_tag = use_signal(|| false);
    let mut filter_rarity = use_signal(|| default_filters.rarity.clone());
    let mut disable_filter_rarity = use_signal(|| false);
    let mut filter_release = use_signal(|| default_filters.release);
    let mut disable_filter_release = use_signal(|| false);

    let update_filter = move |event: Event<FormData>| {
        let filter = event.value();
        *cards_filter.write() = filter.clone();
        *card_amount.write() = CARD_INCREMENT;
        // scroll to top, after updating the filter, to show the first cards
        if let Some(container) = container_ref.write().as_mut() {
            scroll_to_top(container);
        }

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

        let filters = if show_inputs {
            let mut texts = TextFilter::from_text_field(&filter_text);
            texts.extend(filter_tag.to_text_filters());
            Filters {
                texts,
                card_type: filter_type.to_owned(),
                color: filter_color.to_owned(),
                bloom_level: filter_bloom_level.to_owned(),
                rarity: filter_rarity.to_owned(),
                release: filter_release.to_owned(),
            }
        } else {
            default_filters.clone()
        };

        filter_cards(&all_cards, filters)
    });

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
        if show_inputs {
            // Card search
            div { class: "field",
                label { "for": "card_search_{popup_id}", class: "label", "Card search" }
                div { class: "control",
                    input {
                        id: "card_search_{popup_id}",
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
                            label { "for": "card_type_{popup_id}", class: "label", "Card type" }
                            div { class: "control",
                                div { class: "select",
                                    select {
                                        id: "card_type_{popup_id}",
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
                                            if let Some(container) = container_ref.write().as_mut() {
                                                scroll_to_top(container);
                                            }
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
                            label {
                                "for": "card_color_{popup_id}",
                                class: "label",
                                "Color"
                            }
                            div { class: "control",
                                div { class: "select",
                                    select {
                                        id: "card_color_{popup_id}",
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
                                            if let Some(container) = container_ref.write().as_mut() {
                                                scroll_to_top(container);
                                            }
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
                            label {
                                "for": "card_bloom_level_{popup_id}",
                                class: "label",
                                "Bloom level"
                            }
                            div { class: "control",
                                div { class: "select",
                                    select {
                                        id: "card_bloom_level_{popup_id}",
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
                                            if let Some(container) = container_ref.write().as_mut() {
                                                scroll_to_top(container);
                                            }
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
                            label { "for": "card_tag_{popup_id}", class: "label", "Tag" }
                            div { class: "control",
                                div { class: "select",
                                    select {
                                        id: "card_tag_{popup_id}",
                                        disabled: *disable_filter_tag.read(),
                                        oninput: move |ev| {
                                            *filter_tag.write() = match ev.value().as_str() {
                                                "all" => FilterTag::All,
                                                _ => FilterTag::Tag(ev.value()),
                                            };
                                            if let Some(container) = container_ref.write().as_mut() {
                                                scroll_to_top(container);
                                            }
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
                            label {
                                "for": "card_rarity_{popup_id}",
                                class: "label",
                                "Rarity"
                            }
                            div { class: "control",
                                div { class: "select",
                                    select {
                                        id: "card_rarity_{popup_id}",
                                        disabled: *disable_filter_rarity.read(),
                                        oninput: move |ev| {
                                            *filter_rarity.write() = match ev.value().as_str() {
                                                "all" => FilterRarity::All,
                                                "no_alt" => FilterRarity::NoAlternateArt,
                                                _ => FilterRarity::Rarity(ev.value()),
                                            };
                                            // only filter popup when looking for unique cards
                                            *GLOBAL_RARITY.write() = match *filter_rarity.read() {
                                                FilterRarity::NoAlternateArt => FilterRarity::NoAlternateArt,
                                                _ => FilterRarity::All,
                                            };
                                            if let Some(container) = container_ref.write().as_mut() {
                                                scroll_to_top(container);
                                            }
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
                            label {
                                "for": "card_release_{popup_id}",
                                class: "label",
                                "Release"
                            }
                            div { class: "control",
                                div { class: "select",
                                    select {
                                        id: "card_release_{popup_id}",
                                        disabled: *disable_filter_release.read(),
                                        oninput: move |ev| {
                                            *filter_release.write() = match ev.value().as_str() {
                                                "jp" => FilterRelease::Japanese,
                                                "en" => FilterRelease::English,
                                                "unreleased" => FilterRelease::Unreleased,
                                                _ => FilterRelease::All,
                                            };
                                            // only filter popup by specific releases
                                            *GLOBAL_RELEASE.write() = match *filter_release.read() {
                                                FilterRelease::Japanese => FilterRelease::Japanese,
                                                FilterRelease::English => FilterRelease::English,
                                                _ => FilterRelease::All,
                                            };
                                            *card_lang.write() = match *filter_release.read() {
                                                FilterRelease::Japanese => CardLanguage::Japanese,
                                                FilterRelease::English => CardLanguage::English,
                                                FilterRelease::Unreleased => CardLanguage::Japanese,
                                                _ => CardLanguage::English,
                                            };
                                            if let Some(container) = container_ref.write().as_mut() {
                                                scroll_to_top(container);
                                            }
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
                                    *GLOBAL_RARITY.write() = FilterRarity::All;
                                    *filter_release.write() = FilterRelease::All;
                                    *disable_filter_release.write() = false;
                                    *GLOBAL_RELEASE.write() = FilterRelease::All;
                                    *cards_filter.write() = String::new();
                                    *card_amount.write() = CARD_INCREMENT;
                                    if let Some(container) = container_ref.write().as_mut() {
                                        scroll_to_top(container);
                                    }
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
            onmount: move |elem| {
                *container_ref.write() = Some(elem.as_web_event());
            },
            class: "block is-flex is-flex-wrap-wrap is-justify-content-center",
            style: "max-height: 65vh; overflow: scroll;",
            // automatically load more cards when scrolled to the bottom
            onscroll: move |_| {
                if let Some(container) = container_ref.read().as_ref()
                    && is_scrolled_to_bottom(container)
                {
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

#[component]
pub fn CardSearchPopup(popup_id: usize, default_filters: Filters) -> Element {
    rsx! {
        ModelPopup {
            popup_id,
            title: rsx! {
                CardSearchTitle { filters: default_filters.clone() }
            },
            content: rsx! {
                CardSearch {
                    popup_id,
                    db: CARDS_DB.signal(),
                    common_deck: COMMON_DECK.signal(),
                    is_edit: EDIT_DECK.signal(),
                    default_filters,
                    show_inputs: false,
                }
            },
            modal_class: Some("card-search-modal".into()),
        }
    }
}

#[component]
pub fn CardSearchTitle(filters: Filters) -> Element {
    let title = filters
        .texts
        .first()
        .map(|f| f.text.0.clone())
        .unwrap_or("Card search".into());

    let mut subtitles = Vec::new();
    if filters.release == FilterRelease::Japanese {
        subtitles.push("Japanese");
    }
    if filters.release == FilterRelease::English {
        subtitles.push("English");
    }
    if filters.rarity == FilterRarity::NoAlternateArt {
        subtitles.push("No alternate art");
    }
    let subtitle = subtitles.join(", ");

    rsx! {
        h4 {
            span { class: "title", "{title}" }
            if !subtitle.is_empty() {
                span { class: "subtitle", " ({subtitle})" }
            }
        }

    }
}
