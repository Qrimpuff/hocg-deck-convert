use std::{error::Error, ops::Not};

use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use serde::{Deserialize, Serialize};

use crate::download_file;

use super::{CardsInfoMap, CommonCards, CommonCardsConversion, CommonDeck, CommonDeckConversion};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCard(String, u32, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    #[serde(skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,
    oshi: OshiCard,
    deck: Vec<DeckCard>,
    cheer_deck: Vec<DeckCard>,
}

impl Deck {
    pub fn from_file(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }

    pub fn from_text(text: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn to_file(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(serde_json::to_vec(&self)?)
    }

    pub fn to_text(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl CommonCardsConversion for OshiCard {
    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self {
        OshiCard(cards.card_number.clone(), cards.art_order(map))
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, value.1, 1, map)
    }
}

impl CommonCardsConversion for DeckCard {
    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self {
        DeckCard(
            cards.card_number.clone(),
            cards.amount,
            cards.art_order(map),
        )
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, value.2, value.1, map)
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, map: &CardsInfoMap) -> Self {
        Deck {
            deck_name: deck.name,
            oshi: OshiCard::from_common_cards(deck.oshi, map),
            deck: deck
                .main_deck
                .into_iter()
                .map(|c| DeckCard::from_common_cards(c, map))
                .collect(),
            cheer_deck: deck
                .cheer_deck
                .into_iter()
                .map(|c| DeckCard::from_common_cards(c, map))
                .collect(),
        }
    }

    fn to_common_deck(value: Self, map: &CardsInfoMap) -> CommonDeck {
        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: OshiCard::to_common_cards(value.oshi, map),
            main_deck: value
                .deck
                .into_iter()
                .map(|c| DeckCard::to_common_cards(c, map))
                .collect(),
            cheer_deck: value
                .cheer_deck
                .into_iter()
                .map(|c| DeckCard::to_common_cards(c, map))
                .collect(),
        }
    }
}

#[component]
pub fn Import(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    let mut deck_error = use_signal(String::new);
    let mut json = use_signal(String::new);
    let mut file_name = use_signal(String::new);

    let from_text = move |event: Event<FormData>| {
        *json.write() = event.value().clone();
        *common_deck.write() = None;
        *deck_error.write() = "".into();
        *file_name.write() = "".into();
        if event.value().is_empty() {
            return;
        }

        let deck = Deck::from_text(&event.value());
        info!("{:?}", deck);
        match deck {
            Ok(deck) => *common_deck.write() = Some(Deck::to_common_deck(deck, &map.read())),
            Err(e) => *deck_error.write() = e.to_string(),
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
                    let deck = Deck::from_file(&contents);
                    info!("{:?}", deck);
                    match deck {
                        Ok(deck) => {
                            *common_deck.write() = Some(Deck::to_common_deck(deck, &map.read()));
                            match String::from_utf8(contents) {
                                Ok(contents) => *json.write() = contents,
                                Err(e) => *deck_error.write() = e.to_string(),
                            }
                        }
                        Err(e) => *deck_error.write() = e.to_string(),
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
                    label { "for": "holodelta_import_file", class: "file-label",
                        input {
                            id: "holodelta_import_file",
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
            label { "for": "holodelta_import_json", class: "label", "holoDelta json" }
            div { class: "control",
                textarea {
                    id: "holodelta_import_json",
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
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    let mut deck_error = use_signal(String::new);

    let deck: Option<Deck> = common_deck
        .read()
        .as_ref()
        .map(|d| Deck::from_common_deck(d.clone(), &map.read()));
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
                Deck::from_common_deck(d.clone(), &map.read()),
            )
        });
        if let Some((file_name, deck)) = deck {
            let file_name = format!("{}.holodelta.json", file_name.unwrap_or("deck".into()));
            match deck.to_file() {
                Ok(file) => download_file(&file_name, &file[..]),
                Err(e) => *deck_error.write() = e.to_string(),
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
            label { "for": "holodelta_export_json", class: "label", "holoDelta json" }
            div { class: "control",
                textarea {
                    id: "holodelta_export_json",
                    class: "textarea",
                    readonly: true,
                    value: "{text}"
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}
