#![allow(non_snake_case)]

mod components;
mod sources;
mod tracker;

use components::{card_details::CardDetailsPopup, deck_preview::DeckPreview};
use dioxus::{logger::tracing::debug, prelude::*};
use gloo::{
    file::{Blob, BlobContents},
    utils::document,
};
use hocg_fan_sim_assets_model::CardsDatabase;
use price_check::PriceCache;
use serde::Serialize;
use sources::*;
use tracker::{EventType, track_event, track_url};
use wasm_bindgen::prelude::*;
use web_sys::Url;

const HOCG_DECK_CONVERT_API: &str = "https://hocg-deck-convert-api.onrender.com";

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    done_loading();

    let _cards_db: Coroutine<()> = use_coroutine(|_rx| async move {
        *CARDS_DB.write() =
            reqwest::get("https://qrimpuff.github.io/hocg-fan-sim-assets/hocg_cards.json")
                .await
                .unwrap()
                .json()
                .await
                .unwrap()
    });

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
                            href: "https://holodelta.net/",
                            onclick: |_| { track_url("holoDelta") },
                            target: "_blank",
                            "holoDelta"
                        }
                        ", "
                        // a {
                        //     href: "https://daktagames.itch.io/holoduel",
                        //     onclick: |_| { track_url("HoloDuel") },
                        //     target: "_blank",
                        //     "HoloDuel"
                        // }
                        a {
                            href: "https://steamcommunity.com/sharedfiles/filedetails/?id=3302530285",
                            onclick: |_| { track_url("Tabletop Simulator") },
                            target: "_blank",
                            "Tabletop Simulator"
                        }
                        ", or even print proxy sheets."
                    }
                    p { class: "is-hidden-mobile",
                        "To build your deck from scratch, use "
                        a {
                            href: "#",
                            role: "button",
                            onclick: move |evt| {
                                evt.prevent_default();
                                *EDIT_DECK.write() = true;
                                track_url("Edit deck");
                            },
                            "Edit deck"
                        }
                        ". You can also choose one of the "

                        a {
                            href: "#",
                            role: "button",
                            onclick: move |evt| {
                                evt.prevent_default();
                                *EDIT_DECK.write() = false;
                                *COMMON_DECK.write() = Default::default();
                                *SHOW_PRICE.write() = false;
                                *IMPORT_FORMAT.write() = Some(DeckType::StarterDecks);
                                track_url("Official starter decks");
                            },
                            "official starter decks"
                        }
                        " to get you started."
                    }
                    p {
                        "If you have any questions about the game, or this tool, the "
                        a {
                            href: "https://discord.com/invite/GJ9RhA22nP",
                            target: "_blank",
                            onclick: |_| { track_url("Discord - Hololive OCG Fan Server") },
                            span { class: "icon",
                                i { class: "fa-brands fa-discord" }
                            }
                            "Hololive OCG Fan Server"
                        }
                        " is welcoming."
                    }
                }
                div { class: "columns is-tablet",
                    div { class: "column is-two-fifths",
                        Form { card_lang: CARD_LANG.signal() }
                    }
                    div { class: "column is-three-fifths",
                        DeckPreview {
                            card_lang: CARD_LANG.signal(),
                            db: CARDS_DB.signal(),
                            common_deck: COMMON_DECK.signal(),
                            is_edit: EDIT_DECK.signal(),
                            show_price: SHOW_PRICE.signal(),
                            prices: CARDS_PRICES.signal(),
                        }
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

            // card details popup
            if let Some((card, card_type)) = CARD_DETAILS.read().as_ref() {
                CardDetailsPopup { card: card.clone(), card_type: *card_type }
            }
        }
    }
}

static CARD_LANG: GlobalSignal<CardLanguage> = Signal::global(|| CardLanguage::Japanese);
static CARDS_DB: GlobalSignal<CardsDatabase> = Signal::global(Default::default);
static CARDS_PRICES: GlobalSignal<PriceCache> = Signal::global(Default::default);
static COMMON_DECK: GlobalSignal<CommonDeck> = Signal::global(Default::default);
static IMPORT_FORMAT: GlobalSignal<Option<DeckType>> = Signal::global(|| None);
static EXPORT_FORMAT: GlobalSignal<Option<DeckType>> = Signal::global(|| None);
static EDIT_DECK: GlobalSignal<bool> = Signal::global(|| false);
static SHOW_PRICE: GlobalSignal<bool> = Signal::global(|| false);
static SHOW_CARD_DETAILS: GlobalSignal<bool> = Signal::global(|| false);
static CARD_DETAILS: GlobalSignal<Option<(CommonCard, CardType)>> = Signal::global(|| None);

#[component]
fn Form(card_lang: Signal<CardLanguage>) -> Element {
    let mut import_format: Signal<Option<DeckType>> = IMPORT_FORMAT.signal();
    let mut export_format = EXPORT_FORMAT.signal();
    use_effect(move || {
        import_format.set(Some(DeckType::DeckLog));
        export_format.set(Some(DeckType::HoloDelta));
    });

    rsx! {
        form { class: "box",
            div { class: "mb-4 is-flex is-justify-content-center",
                div { class: "buttons has-addons",
                    button {
                        class: "button",
                        class: if !*EDIT_DECK.read() { "is-link is-selected" },
                        r#type: "button",
                        onclick: |_| { *EDIT_DECK.write() = false },
                        span { class: "icon is-small",
                            i { class: "fa-solid fa-file-arrow-down" }
                        }
                        span { "Import deck " }
                    }
                    button {
                        class: "button",
                        class: if *EDIT_DECK.read() { "is-link is-selected" },
                        r#type: "button",
                        onclick: |_| { *EDIT_DECK.write() = true },
                        span { class: "icon is-small",
                            i { class: "fa-solid fa-pen-to-square" }
                        }
                        span { "Edit deck" }
                    }
                }
            }

            if *EDIT_DECK.read() {
                edit_deck::Import {
                    common_deck: COMMON_DECK.signal(),
                    db: CARDS_DB.signal(),
                    is_edit: EDIT_DECK.signal(),
                    show_price: SHOW_PRICE.signal(),
                }
            } else {
                div { class: "field",
                    label { "for": "import_format", class: "label", "Import format" }
                    div { class: "control",
                        div { class: "select",
                            select {
                                id: "import_format",
                                oninput: move |ev| {
                                    *COMMON_DECK.write() = Default::default();
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
                                option {
                                    value: "starter_decks",
                                    selected: *import_format.read() == Some(DeckType::DeckLog),
                                    "Starter decks"
                                }
                                option {
                                    value: "deck_log",
                                    selected: *import_format.read() == Some(DeckType::DeckLog),
                                    "Deck Log (Bushiroad)"
                                }
                                option {
                                    value: "holo_delta",
                                    selected: *import_format.read() == Some(DeckType::HoloDelta),
                                    "holoDelta"
                                }
                                option {
                                    value: "holo_duel",
                                    selected: *import_format.read() == Some(DeckType::HoloDuel),
                                    "HoloDuel"
                                }
                                option {
                                    value: "hocg_tts",
                                    selected: *import_format.read() == Some(DeckType::TabletopSim),
                                    "Tabletop Simulator (by Noodlebrain)"
                                }
                                option {
                                    value: "unknown",
                                    selected: *import_format.read() == Some(DeckType::Unknown),
                                    "I don't know..."
                                }
                            }
                        }
                    }
                }

                div {
                    if *import_format.read() == Some(DeckType::StarterDecks) {
                        starter_decks::Import {
                            common_deck: COMMON_DECK.signal(),
                            db: CARDS_DB.signal(),
                            show_price: SHOW_PRICE.signal(),
                        }
                    }
                    if *import_format.read() == Some(DeckType::DeckLog) {
                        deck_log::Import {
                            common_deck: COMMON_DECK.signal(),
                            db: CARDS_DB.signal(),
                            show_price: SHOW_PRICE.signal(),
                        }
                    }
                    if *import_format.read() == Some(DeckType::HoloDelta) {
                        holodelta::Import {
                            common_deck: COMMON_DECK.signal(),
                            db: CARDS_DB.signal(),
                            show_price: SHOW_PRICE.signal(),
                        }
                    }
                    if *import_format.read() == Some(DeckType::HoloDuel) {
                        holoduel::Import {
                            common_deck: COMMON_DECK.signal(),
                            db: CARDS_DB.signal(),
                            show_price: SHOW_PRICE.signal(),
                        }
                    }
                    if *import_format.read() == Some(DeckType::TabletopSim) {
                        tabletop_sim::Import {
                            common_deck: COMMON_DECK.signal(),
                            db: CARDS_DB.signal(),
                            show_price: SHOW_PRICE.signal(),
                        }
                    }
                    if *import_format.read() == Some(DeckType::Unknown) {
                        UnknownImport {
                            common_deck: COMMON_DECK.signal(),
                            db: CARDS_DB.signal(),
                            show_price: SHOW_PRICE.signal(),
                        }
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
                            option {
                                value: "deck_log",
                                selected: *export_format.read() == Some(DeckType::DeckLog),
                                "Deck Log (Bushiroad)"
                            }
                            option {
                                value: "holo_delta",
                                selected: *export_format.read() == Some(DeckType::HoloDelta),
                                "holoDelta"
                            }
                            option {
                                value: "holo_duel",
                                selected: *export_format.read() == Some(DeckType::HoloDuel),
                                "HoloDuel"
                            }
                            option {
                                value: "hocg_tts",
                                selected: *export_format.read() == Some(DeckType::TabletopSim),
                                "Tabletop Simulator (by Noodlebrain)"
                            }
                            option {
                                value: "proxy_sheets",
                                selected: *export_format.read() == Some(DeckType::ProxySheets),
                                "Proxy sheets (PDF)"
                            }
                            option {
                                value: "price_check",
                                selected: *export_format.read() == Some(DeckType::PriceCheck),
                                "Price check (JPY)"
                            }
                        }
                    }
                }
            }

            div {
                if *export_format.read() == Some(DeckType::DeckLog) {
                    deck_log::Export {
                        common_deck: COMMON_DECK.signal(),
                        db: CARDS_DB.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::HoloDelta) {
                    holodelta::Export {
                        common_deck: COMMON_DECK.signal(),
                        db: CARDS_DB.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::HoloDuel) {
                    holoduel::Export {
                        common_deck: COMMON_DECK.signal(),
                        db: CARDS_DB.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::TabletopSim) {
                    tabletop_sim::Export {
                        common_deck: COMMON_DECK.signal(),
                        db: CARDS_DB.signal(),
                    }
                }
                if *export_format.read() == Some(DeckType::ProxySheets) {
                    proxy_sheets::Export {
                        common_deck: COMMON_DECK.signal(),
                        db: CARDS_DB.signal(),
                        card_lang,
                    }
                }
                if *export_format.read() == Some(DeckType::PriceCheck) {
                    price_check::Export {
                        common_deck: COMMON_DECK.signal(),
                        db: CARDS_DB.signal(),
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
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
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
        *common_deck.write() = Default::default();
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
                        *common_deck.write() = holodelta::Deck::to_common_deck(deck, &db.read());
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
                        *common_deck.write() = holoduel::Deck::to_common_deck(deck, &db.read());
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
                        *common_deck.write() = tabletop_sim::Deck::to_common_deck(deck, &db.read());
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
