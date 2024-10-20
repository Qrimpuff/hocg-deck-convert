use std::{
    error::Error,
    time::{Duration, Instant},
};

use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use serde::Serialize;

use crate::{download_file, track_convert_event, EventType};

use super::{
    holodelta, holoduel, tabletop_sim, CardsInfoMap, CommonDeck, CommonDeckConversion, DeckType,
};

#[derive(Debug, Clone)]
enum Deck {
    HoloDelta(holodelta::Deck),
    HoloDuel(holoduel::Deck),
    TabletopSim(tabletop_sim::Deck),
}

impl Deck {
    pub fn from_file(deck_type: DeckType, bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_file(bytes)?),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_file(bytes)?),
            DeckType::TabletopSim => Deck::TabletopSim(tabletop_sim::Deck::from_file(bytes)?),
            _ => unreachable!("this is not a json deck"),
        })
    }

    pub fn from_text(deck_type: DeckType, text: &str) -> Result<Self, Box<dyn Error>> {
        Ok(match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_text(text)?),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_text(text)?),
            DeckType::TabletopSim => Deck::TabletopSim(tabletop_sim::Deck::from_text(text)?),
            _ => unreachable!("this is not a json deck"),
        })
    }

    pub fn to_file(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(match self {
            Deck::HoloDelta(deck) => deck.to_file()?,
            Deck::HoloDuel(deck) => deck.to_file()?,
            Deck::TabletopSim(deck) => deck.to_file()?,
        })
    }

    pub fn to_text(&self) -> Result<String, Box<dyn Error>> {
        Ok(match self {
            Deck::HoloDelta(deck) => deck.to_text()?,
            Deck::HoloDuel(deck) => deck.to_text()?,
            Deck::TabletopSim(deck) => deck.to_text()?,
        })
    }

    fn from_common_deck(deck_type: DeckType, deck: CommonDeck, map: &CardsInfoMap) -> Self {
        match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_common_deck(deck, map)),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_common_deck(deck, map)),
            DeckType::TabletopSim => {
                Deck::TabletopSim(tabletop_sim::Deck::from_common_deck(deck, map))
            }
            _ => unreachable!("this is not a json deck"),
        }
    }

    fn to_common_deck(value: Self, map: &CardsInfoMap) -> CommonDeck {
        match value {
            Deck::HoloDelta(deck) => holodelta::Deck::to_common_deck(deck, map),
            Deck::HoloDuel(deck) => holoduel::Deck::to_common_deck(deck, map),
            Deck::TabletopSim(deck) => tabletop_sim::Deck::to_common_deck(deck, map),
        }
    }
}

#[component]
pub fn JsonImport(
    deck_type: DeckType,
    import_name: String,
    mut common_deck: Signal<Option<CommonDeck>>,
    map: Signal<CardsInfoMap>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let import_id = import_name.to_lowercase();

    let import_name = use_signal(|| import_name);
    let mut deck_error = use_signal(String::new);
    let mut json = use_signal(String::new);
    let mut file_name = use_signal(String::new);
    let mut tracking_sent: Signal<Option<Instant>> = use_signal(|| None);

    let from_text = move |event: Event<FormData>| {
        *json.write() = event.value().clone();
        *common_deck.write() = None;
        *deck_error.write() = "".into();
        *file_name.write() = "".into();
        if event.value().is_empty() {
            return;
        }

        let deck = Deck::from_text(deck_type, &event.value());
        info!("{:?}", deck);
        match deck {
            Ok(deck) => {
                *common_deck.write() = Some(Deck::to_common_deck(deck, &map.read()));
                if tracking_sent
                    .read()
                    .as_ref()
                    .map(|t| t.elapsed() >= Duration::from_secs(5))
                    .unwrap_or(true)
                {
                    track_convert_event(
                        EventType::Import,
                        EventData {
                            format: import_name.read().clone(),
                            error: None,
                        },
                    );
                    *tracking_sent.write() = Some(Instant::now());
                }
            }
            Err(e) => {
                *deck_error.write() = e.to_string();
                if tracking_sent
                    .read()
                    .as_ref()
                    .map(|t| t.elapsed() >= Duration::from_secs(5))
                    .unwrap_or(true)
                {
                    track_convert_event(
                        EventType::Import,
                        EventData {
                            format: import_name.read().clone(),
                            error: Some(e.to_string()),
                        },
                    );
                    *tracking_sent.write() = Some(Instant::now());
                }
            }
        }
    };

    let from_file = move |event: Event<FormData>| async move {
        *common_deck.write() = None;
        *deck_error.write() = "".into();
        *json.write() = "".into();
        *file_name.write() = "".into();
        if let Some(file_engine) = event.files() {
            let files = file_engine.files();
            for file in &files {
                *file_name.write() = file.clone();

                if let Some(contents) = file_engine.read_file(file).await {
                    let deck = Deck::from_file(deck_type, &contents);
                    info!("{:?}", deck);
                    match deck {
                        Ok(deck) => {
                            *common_deck.write() = Some(Deck::to_common_deck(deck, &map.read()));
                            match String::from_utf8(contents) {
                                Ok(contents) => {
                                    *json.write() = contents;
                                    track_convert_event(
                                        EventType::Import,
                                        EventData {
                                            format: import_name.read().clone(),
                                            error: None,
                                        },
                                    );
                                }
                                Err(e) => {
                                    *deck_error.write() = e.to_string();

                                    track_convert_event(
                                        EventType::Import,
                                        EventData {
                                            format: import_name.read().clone(),
                                            error: Some(e.to_string()),
                                        },
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            *deck_error.write() = e.to_string();
                            track_convert_event(
                                EventType::Import,
                                EventData {
                                    format: import_name.read().clone(),
                                    error: Some(e.to_string()),
                                },
                            );
                        }
                    }
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
                    label { "for": "{import_id}_import_file", class: "file-label",
                        input {
                            id: "{import_id}_import_file",
                            r#type: "file",
                            class: "file-input",
                            accept: ".json",
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
        }
        div { class: "field",
            label { "for": "{import_id}_import_json", class: "label", "{import_name} json" }
            div { class: "control",
                textarea {
                    id: "{import_id}_import_json",
                    class: "textarea",
                    autocomplete: "off",
                    autocapitalize: "off",
                    spellcheck: "false",
                    oninput: from_text,
                    value: "{json}"
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}

#[component]
pub fn JsonExport(
    deck_type: DeckType,
    export_name: String,
    export_id: String,
    mut common_deck: Signal<Option<CommonDeck>>,
    map: Signal<CardsInfoMap>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let export_extension = format!("{export_id}.json");

    let export_name = use_signal(|| export_name);
    let mut deck_error = use_signal(String::new);

    let deck: Option<Deck> = common_deck
        .read()
        .as_ref()
        .map(|d| Deck::from_common_deck(deck_type, d.clone(), &map.read()));
    info!("{:?}", deck);
    let text = match deck {
        Some(deck) => match deck.to_text() {
            Ok(text) => text,
            Err(e) => {
                *deck_error.write() = e.to_string();
                "".into()
            }
        },
        None => "".into(),
    };

    let download_file = move |_| {
        let deck: Option<_> = common_deck.read().as_ref().map(|d| {
            (
                d.file_name(),
                Deck::from_common_deck(deck_type, d.clone(), &map.read()),
            )
        });
        if let Some((file_name, deck)) = deck {
            let file_name = format!("{file_name}.{export_extension}");
            match deck.to_file() {
                Ok(file) => {
                    download_file(&file_name, &file[..]);
                    track_convert_event(
                        EventType::Export,
                        EventData {
                            format: export_name.read().clone(),
                            error: None,
                        },
                    );
                }
                Err(e) => {
                    *deck_error.write() = e.to_string();
                    track_convert_event(
                        EventType::Export,
                        EventData {
                            format: export_name.read().clone(),
                            error: Some(e.to_string()),
                        },
                    );
                }
            }
        }
    };

    rsx! {
        div { class: "field",
            div { class: "control",
                button {
                    class: "button",
                    disabled: common_deck.read().is_none(),
                    r#type: "button",
                    onclick: download_file,
                    span { class: "icon",
                        i { class: "fa-solid fa-download" }
                    }
                    span { "Download deck file" }
                }
            }
        }
        div { class: "field",
            label { "for": "{export_id}_export_json", class: "label", "{export_name} json" }
            div { class: "control",
                textarea {
                    id: "{export_id}_export_json",
                    class: "textarea",
                    readonly: true,
                    value: "{text}"
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}
