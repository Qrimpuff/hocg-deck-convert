#![allow(non_snake_case)]

pub mod sources;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use sources::*;

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        section { class: "section",
            div { class: "container",
                h1 { class: "title", "hololive OCG Deck Converter" }
                div { class: "block",
                    "Convert your hOCG deck between many formats, e.g., Deck Log, holoDelta, HoloDuel, and Tabletop Simulator."
                }
                div { class: "columns is-tablet",
                    div { class: "column is-two-fifths", Form {} }
                    div { class: "column is-three-fifths", DeckPreview {} }
                }
            }
        }
    }
}

static COMMON_DECK: GlobalSignal<Option<CommonDeck>> = Signal::global(|| None);
static DECK_ERROR: GlobalSignal<String> = Signal::global(String::new);

#[component]
fn Form() -> Element {
    rsx! {
        form { class: "box",
            div { class: "field",
                label { class: "label", "Deck Source" }
                div { class: "control",
                    div { class: "select",
                        select {
                            // option { "Deck Log" }
                            // option { "HoloDuel" }
                            // option { "Tabletop Simulator (by Noodlebrain)" }
                            // option { "I don't know..." }
                            option { "holoDelta" }
                        }
                    }
                }
            }

            holodelta::Import { common_deck: COMMON_DECK.signal() }

            // div { class: "field",
            //     div { class: "control",
            //         div { class: "file",
            //             label { class: "file-label",
            //                 input {
            //                     r#type: "file",
            //                     name: "resume",
            //                     class: "file-input"
            //                 }
            //                 span { class: "file-cta",
            //                     span { class: "file-icon",
            //                         i { class: "fa-solid fa-upload" }
            //                     }
            //                     span { class: "file-label", " Choose a file… " }
            //                 }
            //             }
            //         }
            //     }
            // }

            // div { class: "field",
            //     label { class: "label", "Url" }
            //     div { class: "control",
            //         input {
            //             placeholder: "Text input",
            //             r#type: "text",
            //             class: "input"
            //         }
            //     }
            // }

            div { class: "field",
                label { class: "label", "Export format" }
                div { class: "control",
                    div { class: "select",
                        select {
                            // option { "Deck Log" }
                            // option { "HoloDuel" }
                            // option { "Tabletop Simulator (by Noodlebrain)" }
                            option { "holoDelta" }
                        }
                    }
                }
            }

            // div { class: "field",
            //     div { class: "control",
            //         button { class: "button",
            //             span { class: "icon",
            //                 i { class: "fa-solid fa-download" }
            //             }
            //             span { "Download deck file" }
            //         }
            //     }
            // }

            // div { class: "field",
            //     div { class: "control",
            //         button { class: "button",
            //             span { class: "icon",
            //                 i { class: "fa-solid fa-upload" }
            //             }
            //             span { "Upload to Deck Log" }
            //         }
            //     }
            // }

            holodelta::Export { common_deck: COMMON_DECK.signal() }
        }
    }
}

#[component]
fn DeckPreview() -> Element {
    let deck = COMMON_DECK.read();
    let Some(deck) = deck.as_ref() else {
        return rsx! {};
    };

    let oshi = rsx! {
        Card { card: deck.oshi.clone(), card_type: CardType::Oshi }
    };
    let main_deck = deck.main_deck.iter().map(move |card| {
        rsx! {
            Card { card: card.clone(), card_type: CardType::Main }
        }
    });
    let cheer_deck = deck.cheer_deck.iter().map(move |card| {
        rsx! {
            Card { card: card.clone(), card_type: CardType::Cheer }
        }
    });

    rsx! {
        h2 { class: "title is-4 is-spaced", "Deck content" }
        div { class: "block is-flex is-flex-wrap-wrap",
            div { class: "block",
                h3 { class: "subtitle mb-0", "Oshi" }
                div { class: "block is-flex is-flex-wrap-wrap", {oshi} }
            }
            div { class: "block",
                h3 { class: "subtitle mb-0", "Cheer deck" }
                div { class: "block is-flex is-flex-wrap-wrap", {cheer_deck} }
            }
            div { class: "block",
                h3 { class: "subtitle mb-0", "Main deck" }
                div { class: "block is-flex is-flex-wrap-wrap", {main_deck} }
            }
        }
    }
}

#[component]
fn Card(card: CommonCardEntry, card_type: CardType) -> Element {
    let card_number = card.card_number;

    let img_class = if card_type == CardType::Oshi {
        "card-img-oshi"
    } else {
        "card-img"
    };

    let img_path = {
        let set = card_number.split("-").next().unwrap();
        format!("{set}/{card_number}_C.webp")
    };

    let error_img_path = match card_type {
        CardType::Oshi | CardType::Cheer => "cheer-back.webp",
        CardType::Main => "card-back.webp",
    };

    rsx! {
        div {
            figure { class: "image m-2 {img_class}",
                img {
                    title: "{card_number}",
                    border_radius: "3.7%",
                    src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/{img_path}",
                    "onerror": "this.src='https://qrimpuff.github.io/hocg-fan-sim-assets/img/{error_img_path}'"
                }
                if card_type != CardType::Oshi {
                    span { class: "badge is-bottom is-dark", "{card.amount}" }
                }
            }
        }
    }
}
