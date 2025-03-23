use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsInfo;
use num_format::{Locale, ToFormattedString};

use crate::{
    CardLanguage, CardType,
    components::card::Card,
    sources::{CommonDeck, price_check::PriceCache},
};

#[component]
pub fn DeckPreview(
    card_lang: Signal<CardLanguage>,
    info: Signal<CardsInfo>,
    common_deck: Signal<Option<CommonDeck>>,
    show_price: Signal<bool>,
    prices: Signal<PriceCache>,
) -> Element {
    let deck = common_deck.read();

    let Some(deck) = deck.as_ref() else {
        return rsx! {};
    };

    let oshi = rsx! {
        Card {
            card: deck.oshi.clone(),
            card_type: CardType::Oshi,
            card_lang,
            info,
            common_deck,
            show_price,
            prices,
        }
    };
    let main_deck = deck.main_deck.iter().map(move |card| {
        rsx! {
            Card {
                card: card.clone(),
                card_type: CardType::Main,
                card_lang,
                info,
                common_deck,
                show_price,
                prices,
            }
        }
    });
    let cheer_deck = deck.cheer_deck.iter().map(move |card| {
        rsx! {
            Card {
                card: card.clone(),
                card_type: CardType::Cheer,
                card_lang,
                info,
                common_deck,
                show_price,
                prices,
            }
        }
    });

    let info = info.read();
    let mut warnings = deck.validate(&info);

    // warn on missing english proxy
    if *card_lang.read() == CardLanguage::English
        && deck
            .all_cards()
            .any(|c| c.image_path(&info, *card_lang.read()).is_none())
    {
        warnings.push("Missing english proxy.".into());
    }

    let show_price = *show_price.read();
    let approx_price = if show_price
        && deck
            .all_cards()
            .any(|c| c.price(&info, &prices.read()).is_none())
    {
        ">"
    } else {
        ""
    };
    let price = if show_price {
        deck.all_cards()
            .filter_map(|c| c.price(&info, &prices.read()).map(|p| (c, p)))
            .map(|(c, p)| p * c.amount)
            .sum::<u32>()
            .to_formatted_string(&Locale::en)
    } else {
        String::new()
    };

    rsx! {
        if !warnings.is_empty() {
            article { class: "message is-warning",
                div { class: "message-header",
                    p { "Warning" }
                }
                div { class: "message-body content",
                    ul {
                        for warn in warnings {
                            li { "{warn}" }
                        }
                    }
                }
            }
        }
        h2 { class: "title is-4", "Deck preview" }
        p { class: "subtitle is-6 is-spaced",
            if let Some(name) = &deck.name {
                div { "Name: {name}" }
            }
            if show_price {
                div { "Price: {approx_price}¥{price}" }
            }
        }
        div { class: "block is-flex is-flex-wrap-wrap",
            div { class: "block mx-1",
                h3 { class: "subtitle mb-0", "Oshi" }
                div { class: "block is-flex is-flex-wrap-wrap", {oshi} }
            }
            div { class: "block mx-1",
                h3 { class: "subtitle mb-0", "Cheer deck" }
                div { class: "block is-flex is-flex-wrap-wrap", {cheer_deck} }
            }
        }
        div { class: "block mx-1",
            h3 { class: "subtitle mb-0", "Main deck" }
            div { class: "block is-flex is-flex-wrap-wrap", {main_deck} }
        }
    }
}
