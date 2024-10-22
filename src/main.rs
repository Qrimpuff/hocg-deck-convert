#![allow(non_snake_case)]

pub mod sources;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use gloo::{
    file::{Blob, BlobContents},
    utils::{document, format::JsValueSerdeExt},
};
use serde::Serialize;
use sources::*;
use wasm_bindgen::prelude::*;
use web_sys::Url;

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
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
                    "Convert your hOCG deck into various formats, such as Deck Log, holoDelta, HoloDuel, or even proxy sheets."
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
                        span { class: "icon",
                            i { class: "fa-brands fa-github" }
                        }
                        "Qrimpuff"
                    }
                    ". The source code is licensed under "
                    a {
                        href: "https://github.com/Qrimpuff/hocg-deck-convert/blob/main/LICENSE",
                        target: "_blank",
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
                        "hololive Derivative Works guidelines"
                    }
                    ". © 2016 COVER Corp."
                }
                p {
                    "English card translations and proxies are provided by the "
                    a {
                        href: "https://discord.com/invite/GJ9RhA22nP",
                        target: "_blank",
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

static CARDS_INFO: GlobalSignal<CardsInfoMap> = Signal::global(Default::default);
static COMMON_DECK: GlobalSignal<Option<CommonDeck>> = Signal::global(Default::default);

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
                                *import_format
                                    .write() = match ev.value().as_str() {
                                    "deck_log" => Some(DeckType::DeckLog),
                                    "holo_delta" => Some(DeckType::HoloDelta),
                                    "holo_duel" => Some(DeckType::HoloDuel),
                                    "hocg_tts" => Some(DeckType::TabletopSim),
                                    "unknown" => Some(DeckType::Unknown),
                                    _ => None,
                                };
                            },
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
                if *import_format.read() == Some(DeckType::DeckLog) {
                    deck_log::Import { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *import_format.read() == Some(DeckType::HoloDelta) {
                    holodelta::Import { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *import_format.read() == Some(DeckType::HoloDuel) {
                    holoduel::Import { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *import_format.read() == Some(DeckType::TabletopSim) {
                    tabletop_sim::Import { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *import_format.read() == Some(DeckType::Unknown) {
                    UnknownImport { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
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
                                *export_format
                                    .write() = match ev.value().as_str() {
                                    "deck_log" => Some(DeckType::DeckLog),
                                    "holo_delta" => Some(DeckType::HoloDelta),
                                    "holo_duel" => Some(DeckType::HoloDuel),
                                    "hocg_tts" => Some(DeckType::TabletopSim),
                                    "proxy_sheets" => Some(DeckType::ProxySheets),
                                    _ => None,
                                };
                            },
                            option { value: "deck_log", "Deck Log (Bushiroad)" }
                            option { initial_selected: true, value: "holo_delta", "holoDelta" }
                            option { value: "holo_duel", "HoloDuel" }
                            option { value: "hocg_tts", "Tabletop Simulator (by Noodlebrain)" }
                            option { value: "proxy_sheets", "Proxy sheets (PDF)" }
                        }
                    }
                }
            }

            div {
                if *export_format.read() == Some(DeckType::DeckLog) {
                    deck_log::Export { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *export_format.read() == Some(DeckType::HoloDelta) {
                    holodelta::Export { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *export_format.read() == Some(DeckType::HoloDuel) {
                    holoduel::Export { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *export_format.read() == Some(DeckType::TabletopSim) {
                    tabletop_sim::Export { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal() }
                }
                if *export_format.read() == Some(DeckType::ProxySheets) {
                    proxy_sheets::Export { common_deck: COMMON_DECK.signal(), map: CARDS_INFO.signal(), card_lang }
                }
            }
        }
    }
}

#[component]
pub fn UnknownImport(
    mut common_deck: Signal<Option<CommonDeck>>,
    map: Signal<CardsInfoMap>,
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
                    info!("{:?}", deck);
                    if let Ok(deck) = deck {
                        *common_deck.write() =
                            Some(holodelta::Deck::to_common_deck(deck, &map.read()));
                        *deck_success.write() = "Deck file format: holoDelta".into();
                        track_convert_event(
                            EventType::Import,
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
                    info!("{:?}", deck);
                    if let Ok(deck) = deck {
                        *common_deck.write() =
                            Some(holoduel::Deck::to_common_deck(deck, &map.read()));
                        *deck_success.write() = "Deck file format: HoloDuel".into();
                        track_convert_event(
                            EventType::Import,
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
                    info!("{:?}", deck);
                    if let Ok(deck) = deck {
                        *common_deck.write() =
                            Some(tabletop_sim::Deck::to_common_deck(deck, &map.read()));
                        *deck_success.write() =
                            "Deck file format: Tabletop Simulator (by Noodlebrain)".into();
                        track_convert_event(
                            EventType::Import,
                            EventData {
                                format: "Unknown",
                                file_format: Some("Tabletop Sim"),
                                error: None,
                            },
                        );
                        return;
                    }

                    *deck_error.write() = "Cannot parse deck file".into();
                    track_convert_event(
                        EventType::Import,
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
                            onchange: from_file
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
    let map = CARDS_INFO.read();

    let Some(deck) = deck.as_ref() else {
        return rsx! {  };
    };

    let oshi = rsx! {
        Cards { cards: deck.oshi.clone(), card_type: CardType::Oshi, card_lang }
    };
    let main_deck = deck.main_deck.iter().map(move |cards| {
        rsx! {
            Cards { cards: cards.clone(), card_type: CardType::Main, card_lang }
        }
    });
    let cheer_deck = deck.cheer_deck.iter().map(move |cards| {
        rsx! {
            Cards { cards: cards.clone(), card_type: CardType::Cheer, card_lang }
        }
    });

    let mut warnings = deck.validate(&map);

    // warn on missing english proxy
    if *card_lang.read() == CardLanguage::English
        && deck
            .all_cards()
            .any(|c| c.image_path(&map, *card_lang.read()).is_none())
    {
        warnings.push("Missing english proxy.".into());
    }

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
                span { "Name: {name}" }
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

    rsx! {
        div {
            figure { class: "image m-2 {img_class}",
                img {
                    title: "{cards.card_number}",
                    border_radius: "3.7%",
                    src: "{img_path}",
                    "onerror": "this.src='{error_img_path}'"
                }
                if card_type != CardType::Oshi {
                    span { class: "badge is-bottom is-dark", "{cards.amount}" }
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
    }
}

#[wasm_bindgen(module = "/assets/utils.js")]
extern "C" {
    fn track_event(event: &str, data: JsValue);
}

pub enum EventType {
    Import,
    Export,
}

pub fn track_convert_event<T>(event: EventType, data: T)
where
    T: serde::ser::Serialize,
{
    let event = match event {
        EventType::Import => "import",
        EventType::Export => "export",
    };

    let data = JsValue::from_serde(&data).unwrap();

    track_event(event, data);
}
