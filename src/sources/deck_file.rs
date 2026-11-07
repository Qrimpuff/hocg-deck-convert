use std::error::Error;

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use dioxus::{
    logger::tracing::{debug, info},
    prelude::*,
};
use gloo::utils::window;

use crate::app_state::use_persisted;
use serde::Serialize;

use crate::{
    AUTO_SAVE_DECK, CardLanguage, EventType,
    components::deck_validation::DeckValidation,
    download_file,
    sources::{DeckLike, DeckOrPile},
    track_event,
    tracker::TrackEvent,
};

use super::{CardsDatabase, CommonDeck, DeckType, holoburi, holodelta, holoduel, tabletop_sim};

#[derive(Debug, Clone)]
enum Deck {
    HoloDelta(holodelta::Deck),
    HoloDuel(holoduel::Deck),
    TabletopSim(tabletop_sim::Deck),
    HoloBuri(holoburi::Deck),
}

impl Deck {
    pub fn from_file(deck_type: DeckType, bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_file(bytes)?),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_file(bytes)?),
            DeckType::TabletopSim => Deck::TabletopSim(tabletop_sim::Deck::from_file(bytes)?),
            DeckType::HoloBuri => Deck::HoloBuri(holoburi::Deck::from_file(bytes)?),
            _ => unreachable!("this is not a json deck"),
        })
    }

    pub fn from_text(deck_type: DeckType, text: &str) -> Result<Self, Box<dyn Error>> {
        Ok(match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_text(text)?),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_text(text)?),
            DeckType::TabletopSim => Deck::TabletopSim(tabletop_sim::Deck::from_text(text)?),
            DeckType::HoloBuri => Deck::HoloBuri(holoburi::Deck::from_text(text)?),
            _ => unreachable!("this is not a json deck"),
        })
    }

    pub fn to_file(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(match self {
            Deck::HoloDelta(deck) => deck.to_file()?,
            Deck::HoloDuel(deck) => deck.to_file()?,
            Deck::TabletopSim(deck) => deck.to_file()?,
            Deck::HoloBuri(deck) => deck.to_file()?,
        })
    }

    pub fn to_text(&self) -> Result<String, Box<dyn Error>> {
        Ok(match self {
            Deck::HoloDelta(deck) => deck.to_text()?,
            Deck::HoloDuel(deck) => deck.to_text()?,
            Deck::TabletopSim(deck) => deck.to_text()?,
            Deck::HoloBuri(deck) => deck.to_text()?,
        })
    }

    fn from_common_deck(deck_type: DeckType, deck: CommonDeck, db: &CardsDatabase) -> Option<Self> {
        Some(match deck_type {
            DeckType::HoloDelta => Deck::HoloDelta(holodelta::Deck::from_common_deck(deck, db)?),
            DeckType::HoloDuel => Deck::HoloDuel(holoduel::Deck::from_common_deck(deck, db)?),
            DeckType::TabletopSim => {
                Deck::TabletopSim(tabletop_sim::Deck::from_common_deck(deck, db)?)
            }
            DeckType::HoloBuri => Deck::HoloBuri(holoburi::Deck::from_common_deck(deck, db)?),
            _ => unreachable!("this is not a json deck"),
        })
    }

    fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck {
        match value {
            Deck::HoloDelta(deck) => holodelta::Deck::to_common_deck(deck, db),
            Deck::HoloDuel(deck) => holoduel::Deck::to_common_deck(deck, db),
            Deck::TabletopSim(deck) => tabletop_sim::Deck::to_common_deck(deck, db),
            Deck::HoloBuri(deck) => holoburi::Deck::to_common_deck(deck, db),
        }
    }
}

#[component]
pub fn DeckFileImport(
    deck_type: DeckType,
    fallback_deck_type: DeckType,
    import_name: String,
    file_extension: String,
    format_label: String,
    mut common_deck: Signal<DeckOrPile>,
    db: Signal<CardsDatabase>,
    message: Option<Element>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }
    impl TrackEvent for EventData {}

    let import_id = import_name.to_lowercase();

    let import_name = use_signal(|| import_name);
    let mut deck_error = use_signal(String::new);
    let mut json = use_persisted(
        Some(format!("import.{import_id}.{file_extension}")),
        String::new,
    );
    let mut file_name = use_signal(String::new);

    let clear_text = move |_| {
        *json.write() = String::new();
        *deck_error.write() = "".into();
        *file_name.write() = "".into();
    };

    let from_text = move |event: Event<FormData>| {
        *json.write() = event.value().clone();
        *deck_error.write() = "".into();
        *file_name.write() = "".into();
        if event.value().is_empty() {
            return;
        }

        let mut deck = Deck::from_text(deck_type, &event.value());
        if deck.is_err()
            && let Ok(fallback) = Deck::from_text(fallback_deck_type, &event.value())
        {
            info!("fallback to {fallback_deck_type:?}");
            deck = Ok(fallback);
        }
        debug!("{:?}", deck);
        match deck {
            Ok(deck) => {
                *common_deck.write() = DeckOrPile::Deck(Deck::to_common_deck(deck, &db.read()));
                track_event(
                    EventType::Import(import_name.read().clone()),
                    EventData {
                        format: import_name.read().clone(),
                        error: None,
                    },
                );
                AUTO_SAVE_DECK.write().replace(common_deck.read().clone());
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
    };

    let from_file = move |event: Event<FormData>| async move {
        *deck_error.write() = "".into();
        *json.write() = "".into();
        *file_name.write() = "".into();
        let files = event.files();
        for file in &files {
            *file_name.write() = file.name();

            if let Ok(contents) = file.read_bytes().await {
                let contents = contents.to_vec();
                let mut deck = Deck::from_file(deck_type, &contents);
                if deck.is_err()
                    && let Ok(fallback) = Deck::from_file(fallback_deck_type, &contents)
                {
                    info!("fallback to {fallback_deck_type:?}");
                    deck = Ok(fallback);
                }
                debug!("{:?}", deck);
                match deck {
                    Ok(deck) => {
                        *common_deck.write() =
                            DeckOrPile::Deck(Deck::to_common_deck(deck, &db.read()));
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
                                AUTO_SAVE_DECK.write().replace(common_deck.read().clone());
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
                            accept: ".{file_extension}",
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

        {{ message.clone() }}

        div { class: "field",
            label { "for": "{import_id}_import_{file_extension}", class: "label",
                "{import_name} {format_label}"
            }
            div { class: "control",
                textarea {
                    id: "{import_id}_import_{file_extension}",
                    class: "textarea",
                    autocomplete: "off",
                    autocapitalize: "off",
                    spellcheck: "false",
                    oninput: from_text,
                    value: "{json}",
                }
            }
            div { class: "is-flex is-justify-content-space-between is-align-items-center",
                p { class: "help is-danger", "{deck_error}" }
                button {
                    class: "button is-small",
                    disabled: json.read().is_empty(),
                    r#type: "button",
                    onclick: clear_text,
                    span { class: "icon",
                        i { class: "fa-solid fa-xmark" }
                    }
                    span { "Clear" }
                }
            }
        }
    }
}

#[component]
pub fn DeckFileExport(
    deck_type: DeckType,
    export_name: String,
    export_id: String,
    file_extension: String,
    format_label: String,
    allow_unreleased: bool,
    mut common_deck: Signal<DeckOrPile>,
    db: Signal<CardsDatabase>,
    direct_import_url: Option<String>,
    message: Option<Element>,
) -> Element {
    #[derive(Serialize)]
    enum ExportKind {
        Download,
        Copy,
        #[serde(rename = "Direct import")]
        DirectImport,
    }
    #[derive(Serialize)]
    struct EventData {
        format: String,
        export_kind: ExportKind,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }
    impl TrackEvent for EventData {}

    let card_lang = use_signal(|| CardLanguage::Japanese);
    let export_name = use_signal(|| export_name);
    let export_id = use_signal(|| export_id);
    let direct_import_url = use_signal(|| direct_import_url);
    let mut deck_error = use_signal(String::new);
    let mut copied = use_signal(|| false);

    let deck: Option<Deck> = Deck::from_common_deck(
        deck_type,
        common_deck.read().clone().into_deck(&db.read()),
        &db.read(),
    );
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

    let download_extension = file_extension.clone();
    let download_file = move |_| {
        let deck: Option<_> = Deck::from_common_deck(
            deck_type,
            common_deck.read().clone().into_deck(&db.read()),
            &db.read(),
        )
        .map(|d| (common_deck.read().file_name(&db.read()), d));
        if let Some((file_name, deck)) = deck {
            let file_name = format!("{file_name}.{export_id}.{download_extension}");
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

    let direct_import = move |_| {
        if let Some(direct_import_url) = direct_import_url.read().as_ref() {
            let deck: Option<_> = Deck::from_common_deck(
                deck_type,
                common_deck.read().clone().into_deck(&db.read()),
                &db.read(),
            );
            if let Some(deck) = deck {
                match deck.to_file() {
                    Ok(file) => {
                        // The direct import URL takes the file bytes as URL-safe base64.
                        let encoded = BASE64_URL_SAFE_NO_PAD.encode(file.as_slice());
                        let _ = window().open_with_url_and_target(
                            &format!("{direct_import_url}{encoded}"),
                            "_blank",
                        );
                        track_event(
                            EventType::Export(export_name.read().clone()),
                            EventData {
                                format: export_name.read().clone(),
                                export_kind: ExportKind::DirectImport,
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
                                export_kind: ExportKind::DirectImport,
                                error: Some(e.to_string()),
                            },
                        );
                    }
                }
            }
        }
    };

    let copy_text = text.clone();
    let copy_deck_code = move |_| {
        let code = serde_json::to_string(&copy_text).unwrap_or_default();
        let _ = js_sys::eval(&format!("navigator.clipboard.writeText({code})"));
        *copied.write() = true;
        gloo::timers::callback::Timeout::new(2000, move || *copied.write() = false).forget();
        track_event(
            EventType::Export(export_name.read().clone()),
            EventData {
                format: export_name.read().clone(),
                export_kind: ExportKind::Copy,
                error: None,
            },
        );
    };

    rsx! {
        DeckValidation {
            deck_check: true,
            proxy_check: false,
            allow_unreleased,
            allow_pile: false,
            card_lang,
            db,
            common_deck,
        }
        if direct_import_url.read().is_some() {
            div { class: "field",
                div { class: "control",
                    button {
                        class: "button",
                        disabled: text.is_empty(),
                        r#type: "button",
                        onclick: direct_import,
                        span { class: "icon",
                            i { class: "fa-solid fa-arrow-up-right-from-square" }
                        }
                        span { "Directly import into {export_name}" }
                    }
                }
            }
        }
        div { class: "field is-grouped",
            p { class: "control",
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
            p { class: "control",
                button {
                    class: "button",
                    disabled: text.is_empty(),
                    r#type: "button",
                    onclick: copy_deck_code,
                    span { class: "icon",
                        i { class: "fa-solid fa-copy" }
                    }
                    span { "Copy {format_label}" }
                }
            }
        }
        {{ message.clone() }}

        div { class: "field",
            label { "for": "{export_id}_export_{file_extension}", class: "label",
                "{export_name} {format_label}"
            }
            div { class: "control",
                textarea {
                    id: "{export_id}_export_{file_extension}",
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
            if *copied.read() {
                p { class: "help is-success", "Copied to clipboard" }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}
