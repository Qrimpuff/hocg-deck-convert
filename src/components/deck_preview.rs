use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsDatabase;
use serde::Serialize;

use crate::{
    CARDS_PRICES, CardLanguage, CardType, FREE_BASIC_CHEERS, PRICE_SERVICE,
    components::card::Card,
    sources::{CommonDeck, ImageOptions},
    tracker::{EventType, track_event},
};

#[component]
pub fn DeckPreview(
    card_lang: Signal<CardLanguage>,
    image_options: Signal<ImageOptions>,
    db: Signal<CardsDatabase>,
    common_deck: Signal<CommonDeck>,
    is_edit: Signal<bool>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
    }

    let mut deck_name = use_signal(|| {
        common_deck
            .read()
            .name
            .as_ref()
            .cloned()
            .unwrap_or_default()
    });
    // update deck name when importing
    use_effect(move || {
        if !*is_edit.read() {
            deck_name.set(
                common_deck
                    .read()
                    .name
                    .as_ref()
                    .cloned()
                    .unwrap_or_default(),
            );
        }
    });
    let update_deck_name = move |event: Event<FormData>| {
        let deck_name_value = event.value();
        common_deck.write().name =
            Some(deck_name_value.trim().to_string()).filter(|s| !s.is_empty());
        deck_name.set(deck_name_value);

        track_event(
            EventType::EditDeck,
            EventData {
                action: "Update deck name".into(),
            },
        );
    };

    let deck = common_deck.read();

    // Don't render anything if the deck is empty
    if *deck == Default::default() {
        return rsx! {};
    };

    let show_oshi = deck.oshi.is_some();
    let oshi = deck.oshi.iter().map(move |card| {
        rsx! {
            Card {
                card: card.clone(),
                card_type: CardType::Oshi,
                card_lang,
                is_preview: true,
                image_options: *image_options.read(),
                db,
                common_deck,
                is_edit,
                show_price,
            }
        }
    });

    let show_main_deck = !deck.main_deck.is_empty();
    let main_deck = deck.main_deck.iter().map(move |card| {
        rsx! {
            Card {
                card: card.clone(),
                card_type: CardType::Main,
                card_lang,
                is_preview: true,
                image_options: *image_options.read(),
                db,
                common_deck,
                is_edit,
                show_price,
            }
        }
    });

    let show_cheer_deck = !deck.cheer_deck.is_empty();
    let cheer_deck = deck.cheer_deck.iter().map(move |card| {
        rsx! {
            Card {
                card: card.clone(),
                card_type: CardType::Cheer,
                card_lang,
                is_preview: true,
                image_options: *image_options.read(),
                db,
                common_deck,
                is_edit,
                show_price,
            }
        }
    });

    let db = db.read();

    let prices = CARDS_PRICES.read();
    let price_service = *PRICE_SERVICE.read();
    let show_price = *show_price.read();
    let free_basic_cheers = *FREE_BASIC_CHEERS.read();
    let price = if show_price {
        deck.price_display(&db, &prices, price_service, free_basic_cheers)
    } else {
        String::new()
    };

    rsx! {
        h2 { class: "title is-4", "Deck preview" }
        div { class: "subtitle is-6 is-spaced",
            if *is_edit.read() {
                div { class: "control",
                    label {
                        "for": "preview_deck_name",
                        class: "has-text-weight-medium",
                        "Name:"
                    }
                    input {
                        id: "preview_deck_name",
                        class: "deck-name-inline-input",
                        r#type: "text",
                        "aria-label": "Deck name",
                        oninput: update_deck_name,
                        maxlength: 100,
                        placeholder: "Enter a name...",
                        value: "{deck_name}",
                    }
                }
            } else {
                if let Some(name) = &deck.name {
                    div { "Name: {name}" }
                }
            }
        }
        if show_price {
            div { class: "subtitle is-6 is-spaced",
                div { "Price: {price}" }
            }
        }
        if show_oshi || show_cheer_deck {
            div { class: "block is-flex is-flex-wrap-wrap",
                if show_oshi {
                    div { class: "mx-1",
                        h3 { class: "subtitle mb-0", "Oshi" }
                        div { class: "block is-flex is-flex-wrap-wrap", {oshi} }
                    }
                }
                if show_cheer_deck {
                    div { class: "mx-1",
                        h3 { class: "subtitle mb-0", "Cheer deck" }
                        div { class: "block is-flex is-flex-wrap-wrap", {cheer_deck} }
                    }
                }
            }
        }
        if show_main_deck {
            div { class: "block mx-1",
                h3 { class: "subtitle mb-0", "Main deck" }
                div { class: "block is-flex is-flex-wrap-wrap", {main_deck} }
            }
        }
    }
}
