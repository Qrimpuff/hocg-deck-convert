use std::error::Error;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use serde::{Deserialize, Serialize};

use super::{CardsInfoMap, CommonCards, CommonCardsConversion, CommonDeck, CommonDeckConversion};

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

impl CommonCardsConversion for OshiCard {
    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self {
        (cards.card_number.clone(), cards.art_order(map))
    }

    fn to_common_cards(value: Self, map: &CardsInfoMap) -> CommonCards {
        CommonCards::from_card_number_and_art_order(value.0, value.1, 1, map)
    }
}

impl CommonCardsConversion for DeckCard {
    fn from_common_cards(cards: CommonCards, map: &CardsInfoMap) -> Self {
        (
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
            deck_name: deck.deck_name,
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
            deck_name: value.deck_name,
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

    let from_text = move |event: Event<FormData>| {
        *common_deck.write() = None;
        *deck_error.write() = "".into();
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
