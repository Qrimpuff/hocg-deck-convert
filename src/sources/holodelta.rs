use std::error::Error;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use serde::{Deserialize, Serialize};

use super::{CommonCardEntry, CommonDeck};

type OshiCard = (String, u32);
type DeckCard = (String, u32, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    deck_name: String,
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

impl From<CommonCardEntry> for OshiCard {
    fn from(value: CommonCardEntry) -> Self {
        (value.card_number, value.rarity)
    }
}
impl From<OshiCard> for CommonCardEntry {
    fn from(value: OshiCard) -> Self {
        CommonCardEntry {
            card_number: value.0,
            rarity: value.1,
            amount: 1,
        }
    }
}

impl From<CommonCardEntry> for DeckCard {
    fn from(value: CommonCardEntry) -> Self {
        (value.card_number, value.amount, value.rarity)
    }
}
impl From<DeckCard> for CommonCardEntry {
    fn from(value: DeckCard) -> Self {
        CommonCardEntry {
            card_number: value.0,
            rarity: value.2,
            amount: value.1,
        }
    }
}

impl From<CommonDeck> for Deck {
    fn from(value: CommonDeck) -> Self {
        Deck {
            deck_name: value.deck_name,
            oshi: value.oshi.into(),
            deck: value.main_deck.into_iter().map(Into::into).collect(),
            cheer_deck: value.cheer_deck.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<Deck> for CommonDeck {
    fn from(value: Deck) -> Self {
        CommonDeck {
            deck_name: value.deck_name,
            oshi: value.oshi.into(),
            main_deck: value.deck.into_iter().map(Into::into).collect(),
            cheer_deck: value.cheer_deck.into_iter().map(Into::into).collect(),
        }
    }
}

#[component]
pub fn Import(mut common_deck: Signal<Option<CommonDeck>>) -> Element {
    let mut deck_error = use_signal(String::new);

    let from_text = move |event: Event<FormData>| {
        *common_deck.write() = None;
        *deck_error.write() = "".into();
        if event.value().is_empty() {
            return;
        }

        let deck = Deck::from_text(&event.value());
        info!("{:?}", deck);
        match deck {
            Ok(deck) => *common_deck.write() = Some(deck.into()),
            Err(e) => *deck_error.write() = e.to_string(),
        }
    };

    rsx! {
        div { class: "field",
            label { class: "label", "Json" }
            div { class: "control",
                textarea {
                    placeholder: "e.g. Hello world",
                    class: "textarea",
                    oninput: from_text
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>) -> Element {
    let mut deck_error = use_signal(String::new);

    let deck: Option<Deck> = common_deck.read().as_ref().map(|d| d.clone().into());
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

    rsx! {
        div { class: "field",
            label { class: "label", "Json" }
            div { class: "control",
                textarea { class: "textarea", value: "{text}" }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}
