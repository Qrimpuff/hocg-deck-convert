use std::error::Error;

use dioxus::{
    logger::tracing::{debug, info},
    prelude::*,
};
use serde::Serialize;
use web_time::{Duration, Instant};

use crate::{EventType, download_file, track_event};

use super::{
    CardsDatabase, CommonDeck, CommonDeckConversion, DeckType, holodelta, holoduel, tabletop_sim,
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

    fn from_common_deck(deck_type: DeckType, deck: CommonDeck, db: &CardsDatabase) -> Option<Self> {
        Some(match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_common_deck(deck, db)?),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_common_deck(deck, db)?),
            DeckType::TabletopSim => {
                Deck::TabletopSim(tabletop_sim::Deck::from_common_deck(deck, db)?)
            }
            _ => unreachable!("this is not a json deck"),
        })
    }

    fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck {
        match value {
            Deck::HoloDelta(deck) => holodelta::Deck::to_common_deck(deck, db),
            Deck::HoloDuel(deck) => holoduel::Deck::to_common_deck(deck, db),
            Deck::TabletopSim(deck) => tabletop_sim::Deck::to_common_deck(deck, db),
        }
    }
}

#[component]
pub fn JsonImport(
    deck_type: DeckType,
    fallback_deck_type: DeckType,
    import_name: String,
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
    show_price: Signal<bool>,
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
        *common_deck.write() = Default::default();
        *show_price.write() = false;
        *deck_error.write() = "".into();
        *file_name.write() = "".into();
        if event.value().is_empty() {
            return;
        }

        let mut deck = Deck::from_text(deck_type, &event.value());
        if deck.is_err() {
            if let Ok(fallback) = Deck::from_text(fallback_deck_type, &event.value()) {
                info!("fallback to {fallback_deck_type:?}");
                deck = Ok(fallback);
            }
        }
        debug!("{:?}", deck);
        match deck {
            Ok(deck) => {
                *common_deck.write() = Deck::to_common_deck(deck, &db.read());
                *show_price.write() = false;
                if tracking_sent
                    .peek()
                    .as_ref()
                    .map(|t| t.elapsed() >= Duration::from_secs(30))
                    .unwrap_or(true)
                {
                    track_event(
                        EventType::Import(import_name.read().clone()),
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
                    .peek()
                    .as_ref()
                    .map(|t| t.elapsed() >= Duration::from_secs(30))
                    .unwrap_or(true)
                {
                    track_event(
                        EventType::Import(import_name.read().clone()),
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
        *common_deck.write() = Default::default();
        *show_price.write() = false;
        *deck_error.write() = "".into();
        *json.write() = "".into();
        *file_name.write() = "".into();
        if let Some(file_engine) = event.files() {
            let files = file_engine.files();
            for file in &files {
                *file_name.write() = file.clone();

                if let Some(contents) = file_engine.read_file(file).await {
                    let mut deck = Deck::from_file(deck_type, &contents);
                    if deck.is_err() {
                        if let Ok(fallback) = Deck::from_file(fallback_deck_type, &contents) {
                            info!("fallback to {fallback_deck_type:?}");
                            deck = Ok(fallback);
                        }
                    }
                    debug!("{:?}", deck);
                    match deck {
                        Ok(deck) => {
                            *common_deck.write() = Deck::to_common_deck(deck, &db.read());
                            *show_price.write() = false;
                            match String::from_utf8(contents) {
                                Ok(contents) => {
                                    *json.write() = contents;
                                    track_event(
                                        EventType::Import(import_name.read().clone()),
                                        EventData {
                                            format: import_name.read().clone(),
                                            error: None,
                                        },
                                    );
                                }
                                Err(e) => {
                                    *deck_error.write() = e.to_string();
                                    track_event(
                                        EventType::Import(import_name.read().clone()),
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
                            track_event(
                                EventType::Import(import_name.read().clone()),
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
                    value: "{json}",
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
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
) -> Element {
    #[derive(Serialize)]
    enum ExportKind {
        Download,
        Copy,
    }
    #[derive(Serialize)]
    struct EventData {
        format: String,
        export_kind: ExportKind,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let export_name = use_signal(|| export_name);
    let export_id = use_signal(|| export_id);
    let mut deck_error = use_signal(String::new);

    let deck: Option<Deck> =
        Deck::from_common_deck(deck_type, common_deck.read().clone(), &db.read());
    debug!("{:?}", deck);
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
        let deck: Option<_> =
            Deck::from_common_deck(deck_type, common_deck.read().clone(), &db.read())
                .map(|d| (common_deck.read().file_name(), d));
        if let Some((file_name, deck)) = deck {
            let file_name = format!("{file_name}.{export_id}.json");
            match deck.to_file() {
                Ok(file) => {
                    download_file(&file_name, &file[..]);
                    track_event(
                        EventType::Export(export_name.read().clone()),
                        EventData {
                            format: export_name.read().clone(),
                            export_kind: ExportKind::Download,
                            error: None,
                        },
                    );
                }
                Err(e) => {
                    *deck_error.write() = e.to_string();
                    track_event(
                        EventType::Export(export_name.read().clone()),
                        EventData {
                            format: export_name.read().clone(),
                            export_kind: ExportKind::Download,
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
                    disabled: text.is_empty(),
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
                    oncopy: move |_| {
                        track_event(
                            EventType::Export(export_name.read().clone()),
                            EventData {
                                format: export_name.read().clone(),
                                export_kind: ExportKind::Copy,
                                error: None,
                            },
                        );
                    },
                    value: "{text}",
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}
