use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsDatabase;
use num_format::{Locale, ToFormattedString};

use crate::{
    CardLanguage, CardType,
    components::card::Card,
    sources::{CommonDeck, price_check::PriceCache},
};

#[component]
pub fn DeckPreview(
    card_lang: Signal<CardLanguage>,
    db: Signal<CardsDatabase>,
    common_deck: Signal<CommonDeck>,
    is_edit: Signal<bool>,
    show_price: Signal<bool>,
    prices: Signal<PriceCache>,
) -> Element {
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
                db,
                common_deck,
                is_edit,
                show_price,
                prices,
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
                db,
                common_deck,
                is_edit,
                show_price,
                prices,
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
                db,
                common_deck,
                is_edit,
                show_price,
                prices,
            }
        }
    });

    let db = db.read();

    let show_price = *show_price.read();
    let approx_price = if show_price
        && deck
            .all_cards()
            .any(|c| c.price(&db, &prices.read()).is_none())
    {
        ">"
    } else {
        ""
    };
    let price = if show_price {
        deck.all_cards()
            .filter_map(|c| c.price(&db, &prices.read()).map(|p| (c, p)))
            .map(|(c, p)| p * c.amount)
            .sum::<u32>()
            .to_formatted_string(&Locale::en)
    } else {
        String::new()
    };

    rsx! {
        h2 { class: "title is-4", "Deck preview" }
        p { class: "subtitle is-6 is-spaced",
            if let Some(name) = &deck.name {
                div { "Name: {name}" }
            }
            if show_price {
                div { "Price: {approx_price}¥{price}" }
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
