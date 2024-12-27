#![allow(non_snake_case)]

mod sources;
mod tracker;

use dioxus::{logger::tracing::debug, prelude::*};
use gloo::{
    file::{Blob, BlobContents},
    utils::document,
};
use hocg_fan_sim_assets_model::CardsInfo;
use num_format::{Locale, ToFormattedString};
use price_check::PriceCache;
use serde::Serialize;
use sources::*;
use tracker::{track_event, track_url, EventType};
use wasm_bindgen::prelude::*;
use web_sys::Url;

const HOCG_DECK_CONVERT_API: &str = "https://hocg-deck-convert-api-y7os.shuttle.app";

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    done_loading();

    let _card_info: Coroutine<()> = use_coroutine(|_rx| async move {
        *CARDS_INFO.write() =
            reqwest::get("https://qrimpuff.github.io/hocg-fan-sim-assets/cards_info.json")
                .await
                .unwrap()
                .json()
                .await
                .unwrap()
    });

    let card_lang = use_signal(|| CardLanguage::Japanese);

    rsx! {
        section { class: "section",
            div { class: "container",
                h1 { class: "title", "hololive OCG Deck Converter" }
                div { class: "block",
                    p {
                        "Convert your hOCG deck between various formats, such as "
                        a {
                            href: "https://decklog-en.bushiroad.com/ja/create?c=108",
                            target: "_blank",
                            onclick: |_| { track_url("Deck Log") },
                            "Deck Log"
                        }
                        ", "
                        a {
                            href: "https://github.com/GabeJWJ/holoDelta",
                            onclick: |_| { track_url("holoDelta") },
                            target: "_blank",
                            "holoDelta"
                        }
                        ", "
                        a {
                            href: "https://daktagames.itch.io/holoduel",
                            onclick: |_| { track_url("HoloDuel") },
                            target: "_blank",
                            "HoloDuel"
                        }
                        ", or even print proxy sheets."
                    }
                    p { class: "is-hidden-mobile",
                        "Consider using Deck Log to build your deck, or you can also choose one of the official starter decks to get you started."
                    }
                    p {
                        "If you have any questions about the game, the "
                        a {
                            href: "https://discord.com/invite/GJ9RhA22nP",
                            target: "_blank",
                            onclick: |_| { track_url("Discord - Hololive OCG Fan Server") },
                            span { class: "icon",
                                i { class: "fa-brands fa-discord" }
                            }
                            "Hololive OCG Fan Server"
                        }
                        " is welcoming to beginners."
                    }
                }
                div { class: "columns is-tablet",
                    div { class: "column is-two-fifths",
                        Form { card_lang }
                    }
                    div { class: "column is-three-fifths",
                        DeckPreview { card_lang }
                    }
                }
            }
        }
        footer { class: "footer",
            div { class: "content has-text-centered has-text-grey",
                p {
                    "Made by "
                    a {
                        href: "https://github.com/Qrimpuff/hocg-deck-convert",
                        target: "_blank",
                        onclick: |_| { track_url("GitHub - hocg-deck-convert") },
                        span { class: "icon",
                            i { class: "fa-brands fa-github" }
                        }
                        "Qrimpuff"
                    }
                    ". The source code is licensed under "
                    a {
                        href: "https://github.com/Qrimpuff/hocg-deck-convert/blob/main/LICENSE",
                        target: "_blank",
                        onclick: |_| { track_url("GitHub - hocg-deck-convert - license") },
                        "MIT"
                    }
                    "."
                }
                p {
                    "This is a fan website for the hololive Official Card Game and not affiliated with COVER Corp. "
                    "This project was made while following all guidelines under the "
                    a {
                        href: "https://en.hololive.tv/terms",
                        target: "_blank",
                        onclick: |_| { track_url("hololive Derivative Works guidelines") },
                        "hololive Derivative Works guidelines"
                    }
                    ". © 2016 COVER Corp."
                }
                p {
                    "English card translations and proxies are provided by the "
                    a {
                        href: "https://discord.com/invite/GJ9RhA22nP",
                        target: "_blank",
                        onclick: |_| { track_url("Discord - Hololive OCG Fan Server") },
                        span { class: "icon",
                            i { class: "fa-brands fa-discord" }
                        }
                        "Hololive OCG Fan Server"
                    }
                    "."
                }
                p { "Please support the official card game." }
            }
        }
    }
}

static CARDS_INFO: GlobalSignal<CardsInfo> = Signal::global(Default::default);
static CARDS_PRICES: GlobalSignal<PriceCache> = Signal::global(Default::default);
static COMMON_DECK: GlobalSignal<Option<CommonDeck>> = Signal::global(Default::default);
static SHOW_PRICE: GlobalSignal<bool> = Signal::global(|| false);

#[component]
fn Form(card_lang: Signal<CardLanguage>) -> Element {
    let mut import_format = use_signal(|| None);
    let mut export_format = use_signal(|| None);
    use_effect(move || {
        import_format.set(Some(DeckType::DeckLog));
        export_format.set(Some(DeckType::HoloDelta));
    });

    rsx! {
        form { class: "box",
            div { class: "field",
                label { "for": "import_format", class: "label", "Import format" }
                div { class: "control",
                    div { class: "select",
                        select {
                            id: "import_format",
                            oninput: move |ev| {
                                *COMMON_DECK.write() = None;
                                *SHOW_PRICE.write() = false;
                                *import_format.write() = match ev.value().as_str() {
                                    "starter_decks" => Some(DeckType::StarterDecks),
                                    "deck_log" => Some(DeckType::DeckLog),
                                    "holo_delta" => Some(DeckType::HoloDelta),
                                    "holo_duel" => Some(DeckType::HoloDuel),
                                    "hocg_tts" => Some(DeckType::TabletopSim),
                                    "unknown" => Some(DeckType::Unknown),
                                    _ => None,
                                };
                            },
                            option { value: "starter_decks", "Starter decks" }
                            option { initial_selected: true, value: "deck_log", "Deck Log (Bushiroad)" }
                            option { value: "holo_delta", "holoDelta" }
                            option { value: "holo_duel", "HoloDuel" }
                            option { value: "hocg_tts", "Tabletop Simulator (by Noodlebrain)" }
                            option { value: "unknown", "I don't know..." }
                        }
                    }
                }
            }

            div {
                if *import_format.read() == Some(DeckType::StarterDecks) {
                    starter_decks::Import {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        show_price: SHOW_PRICE.signal(),
                    }
                }
                if *import_format.read() == Some(DeckType::DeckLog) {
                    deck_log::Import {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        show_price: SHOW_PRICE.signal(),
                    }
                }
                if *import_format.read() == Some(DeckType::HoloDelta) {
                    holodelta::Import {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        show_price: SHOW_PRICE.signal(),
                    }
                }
                if *import_format.read() == Some(DeckType::HoloDuel) {
                    holoduel::Import {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        show_price: SHOW_PRICE.signal(),
                    }
                }
                if *import_format.read() == Some(DeckType::TabletopSim) {
                    tabletop_sim::Import {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        show_price: SHOW_PRICE.signal(),
                    }
                }
                if *import_format.read() == Some(DeckType::Unknown) {
                    UnknownImport {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        show_price: SHOW_PRICE.signal(),
                    }
                }
            }

            hr {}

            div { class: "field",
                label { "for": "export_format", class: "label", "Export format" }
                div { class: "control",
                    div { class: "select",
                        select {
                            id: "export_format",
                            oninput: move |ev| {
                                *card_lang.write() = CardLanguage::Japanese;
                                *SHOW_PRICE.write() = false;
                                *export_format.write() = match ev.value().as_str() {
                                    "deck_log" => Some(DeckType::DeckLog),
                                    "holo_delta" => Some(DeckType::HoloDelta),
                                    "holo_duel" => Some(DeckType::HoloDuel),
                                    "hocg_tts" => Some(DeckType::TabletopSim),
                                    "proxy_sheets" => Some(DeckType::ProxySheets),
                                    "price_check" => Some(DeckType::PriceCheck),
                                    _ => None,
                                };
                            },
                            option { value: "deck_log", "Deck Log (Bushiroad)" }
                            option { initial_selected: true, value: "holo_delta", "holoDelta" }
                            option { value: "holo_duel", "HoloDuel" }
                            option { value: "hocg_tts", "Tabletop Simulator (by Noodlebrain)" }
                            option { value: "proxy_sheets", "Proxy sheets (PDF)" }
                            option { value: "price_check", "Price check (JPY)" }
                        }
                    }
                }
            }

            div {
                if *export_format.read() == Some(DeckType::DeckLog) {
                    deck_log::Export {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::HoloDelta) {
                    holodelta::Export {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::HoloDuel) {
                    holoduel::Export {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::TabletopSim) {
                    tabletop_sim::Export {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::ProxySheets) {
                    proxy_sheets::Export {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        card_lang,
                    }
                }
                if *export_format.read() == Some(DeckType::PriceCheck) {
                    price_check::Export {
                        common_deck: COMMON_DECK.signal(),
                        info: CARDS_INFO.signal(),
                        prices: CARDS_PRICES.signal(),
                        card_lang,
                        show_price: SHOW_PRICE.signal(),
                    }
                }
            }
        }
    }
}

#[component]
pub fn UnknownImport(
    mut common_deck: Signal<Option<CommonDeck>>,
    info: Signal<CardsInfo>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_format: Option<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let mut deck_error = use_signal(String::new);
    let mut deck_success = use_signal(String::new);
    let mut file_name = use_signal(String::new);

    let from_file = move |event: Event<FormData>| async move {
        *common_deck.write() = None;
        *show_price.write() = false;
        *deck_error.write() = "".into();
        *deck_success.write() = "".into();
        *file_name.write() = "".into();
        if let Some(file_engine) = event.files() {
            let files = file_engine.files();
            for file in &files {
                *file_name.write() = file.clone();

                if let Some(contents) = file_engine.read_file(file).await {
                    // holoDelta
                    let deck = holodelta::Deck::from_file(&contents);
                    debug!("{:?}", deck);
                    if let Ok(deck) = deck {
                        *common_deck.write() =
                            Some(holodelta::Deck::to_common_deck(deck, &info.read()));
                        *deck_success.write() = "Deck file format: holoDelta".into();
                        *show_price.write() = false;
                        track_event(
                            EventType::Import("Unknown".into()),
                            EventData {
                                format: "Unknown",
                                file_format: Some("holoDelta"),
                                error: None,
                            },
                        );
                        return;
                    }

                    // HoloDuel
                    let deck = holoduel::Deck::from_file(&contents);
                    debug!("{:?}", deck);
                    if let Ok(deck) = deck {
                        *common_deck.write() =
                            Some(holoduel::Deck::to_common_deck(deck, &info.read()));
                        *deck_success.write() = "Deck file format: HoloDuel".into();
                        *show_price.write() = false;
                        track_event(
                            EventType::Import("Unknown".into()),
                            EventData {
                                format: "Unknown",
                                file_format: Some("HoloDuel"),
                                error: None,
                            },
                        );
                        return;
                    }

                    // Tabletop Sim
                    let deck = tabletop_sim::Deck::from_file(&contents);
                    debug!("{:?}", deck);
                    if let Ok(deck) = deck {
                        *common_deck.write() =
                            Some(tabletop_sim::Deck::to_common_deck(deck, &info.read()));
                        *deck_success.write() =
                            "Deck file format: Tabletop Simulator (by Noodlebrain)".into();
                        *show_price.write() = false;
                        track_event(
                            EventType::Import("Unknown".into()),
                            EventData {
                                format: "Unknown",
                                file_format: Some("Tabletop Sim"),
                                error: None,
                            },
                        );
                        return;
                    }

                    *deck_error.write() = "Cannot parse deck file".into();
                    track_event(
                        EventType::Import("Unknown".into()),
                        EventData {
                            format: "Unknown",
                            file_format: None,
                            error: Some("Cannot parse deck file".into()),
                        },
                    );
                }
            }
        }
    };

    rsx! {
        div { class: "field",
            div { class: "control",
                div {
                    class: "file",
                    class: if !file_name.read().is_empty() { "has-name" },
                    label { "for": "unknown_import_file", class: "file-label",
                        input {
                            id: "unknown_import_file",
                            r#type: "file",
                            class: "file-input",
                            onchange: from_file,
                        }
                        span { class: "file-cta",
                            span { class: "file-icon",
                                i { class: "fa-solid fa-upload" }
                            }
                            span { class: "file-label", " Load a file… " }
                        }
                        if !file_name.read().is_empty() {
                            span { class: "file-name", "{file_name}" }
                        }
                    }
                }
            }
            p { class: "help is-success", "{deck_success}" }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    Oshi,
    Cheer,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CardLanguage {
    Japanese,
    English,
}

#[component]
fn DeckPreview(card_lang: Signal<CardLanguage>) -> Element {
    let deck = COMMON_DECK.read();
    let info = CARDS_INFO.read();

    let Some(deck) = deck.as_ref() else {
        return rsx! {};
    };

    let oshi = rsx! {
        Cards {
            cards: deck.oshi.clone(),
            card_type: CardType::Oshi,
            card_lang,
        }
    };
    let main_deck = deck.main_deck.iter().map(move |cards| {
        rsx! {
            Cards {
                cards: cards.clone(),
                card_type: CardType::Main,
                card_lang,
            }
        }
    });
    let cheer_deck = deck.cheer_deck.iter().map(move |cards| {
        rsx! {
            Cards {
                cards: cards.clone(),
                card_type: CardType::Cheer,
                card_lang,
            }
        }
    });

    let mut warnings = deck.validate(&info);

    // warn on missing english proxy
    if *card_lang.read() == CardLanguage::English
        && deck
            .all_cards()
            .any(|c| c.image_path(&info, *card_lang.read()).is_none())
    {
        warnings.push("Missing english proxy.".into());
    }

    let show_price = *SHOW_PRICE.read();
    let approx_price = if show_price
        && deck
            .all_cards()
            .any(|c| c.price(&info, &CARDS_PRICES.read()).is_none())
    {
        ">"
    } else {
        ""
    };
    let price = if show_price {
        deck.all_cards()
            .filter_map(|c| c.price(&info, &CARDS_PRICES.read()).map(|p| (c, p)))
            .map(|(c, p)| p * c.amount)
            .sum::<u32>()
            .to_formatted_string(&Locale::en)
    } else {
        String::new()
    };

    rsx! {
        if !warnings.is_empty() {
            article { class: "message is-warning",
                div { class: "message-header",
                    p { "Warning" }
                }
                div { class: "message-body content",
                    ul {
                        for warn in warnings {
                            li { "{warn}" }
                        }
                    }
                }
            }
        }
        h2 { class: "title is-4", "Deck preview" }
        p { class: "subtitle is-6 is-spaced",
            if let Some(name) = &deck.name {
                div { "Name: {name}" }
            }
            if show_price {
                div { "Price: {approx_price}¥{price}" }
            }
        }
        div { class: "block is-flex is-flex-wrap-wrap",
            div { class: "block mx-1",
                h3 { class: "subtitle mb-0", "Oshi" }
                div { class: "block is-flex is-flex-wrap-wrap", {oshi} }
            }
            div { class: "block mx-1",
                h3 { class: "subtitle mb-0", "Cheer deck" }
                div { class: "block is-flex is-flex-wrap-wrap", {cheer_deck} }
            }
        }
        div { class: "block mx-1",
            h3 { class: "subtitle mb-0", "Main deck" }
            div { class: "block is-flex is-flex-wrap-wrap", {main_deck} }
        }
    }
}

#[component]
fn Cards(cards: CommonCards, card_type: CardType, card_lang: Signal<CardLanguage>) -> Element {
    let img_class = if card_type == CardType::Oshi {
        "card-img-oshi"
    } else {
        "card-img"
    };

    let error_img_path: &str = match card_type {
        CardType::Oshi | CardType::Cheer => "cheer-back.webp",
        CardType::Main => "card-back.webp",
    };
    let error_img_path =
        format!("https://qrimpuff.github.io/hocg-fan-sim-assets/img/{error_img_path}");

    let img_path = cards
        .image_path(&CARDS_INFO.read(), *card_lang.read())
        .unwrap_or_else(|| error_img_path.clone());

    let show_price = *SHOW_PRICE.read();
    let price = cards
        .price(&CARDS_INFO.read(), &CARDS_PRICES.read())
        .map(|p| p.to_formatted_string(&Locale::en))
        .unwrap_or("?".into());
    // TODO not only yuyutei
    let price_url = cards
        .card_info(&CARDS_INFO.read())
        .and_then(|c| c.yuyutei_sell_url.clone());

    // verify card amount
    let warning = COMMON_DECK
        .read()
        .as_ref()
        .map(|d| {
            d.all_cards()
                .filter(|c| c.card_number == cards.card_number)
                .map(|c| c.amount)
                .sum::<u32>()
        })
        .unwrap_or(0)
        > cards
            .card_info(&CARDS_INFO.read())
            .map(|i| i.max)
            .unwrap_or(50);
    let warning_class = if warning { "is-warning" } else { "is-dark" };

    rsx! {
        div {
            figure { class: "image m-2 {img_class}",
                img {
                    title: "{cards.card_number}",
                    border_radius: "3.7%",
                    src: "{img_path}",
                    "onerror": "this.src='{error_img_path}'",
                }
                if show_price {
                    span { class: "badge is-bottom {warning_class}",
                        " ¥{price} × {cards.amount} "
                        if let Some(price_url) = price_url {
                            a {
                                title: "Go to Yuyutei for {cards.card_number}",
                                href: "{price_url}",
                                target: "_blank",
                                onclick: |_| { track_url("Yuyutei") },
                                i { class: "fa-solid fa-arrow-up-right-from-square" }
                            }
                        }
                    }
                } else if card_type != CardType::Oshi {
                    span { class: "badge is-bottom {warning_class}", "{cards.amount}" }
                }
            }
        }
    }
}

pub fn download_file(file_name: &str, content: impl BlobContents) {
    let a = document()
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document().body().unwrap().append_child(&a).unwrap();
    a.set_class_name("is-hidden");
    let blob = Blob::new_with_options(content, Some("octet/stream"));
    let url = Url::create_object_url_with_blob(&blob.into()).unwrap();
    a.set_attribute("href", &url).unwrap();
    a.set_attribute("download", file_name).unwrap();
    a.click();
    Url::revoke_object_url(&url).unwrap();
    document().body().unwrap().remove_child(&a).unwrap();
}

pub fn done_loading() {
    if let Some(loading) = document().get_element_by_id("loading") {
        loading.remove();

        track_event(EventType::Entry, Option::<()>::None);
    }
}
