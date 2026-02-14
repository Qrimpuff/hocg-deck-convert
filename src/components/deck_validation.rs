use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsDatabase;

use crate::{
    CardLanguage, CardType,
    sources::{CommonDeck, ImageOptions},
};

pub fn has_missing_proxies(
    deck: &CommonDeck,
    db: &CardsDatabase,
    card_lang: CardLanguage,
) -> bool {
    deck.all_cards()
        .filter(|card| card.card_type(db) != Some(CardType::Cheer))
        .any(|card| {
            card.image_path(db, card_lang, ImageOptions::proxy_validation())
                .is_none()
        })
}

#[component]
pub fn DeckValidation(
    deck_check: bool,
    proxy_check: bool,
    allow_unreleased: bool,
    card_lang: Signal<CardLanguage>,
    db: Signal<CardsDatabase>,
    common_deck: Signal<CommonDeck>,
) -> Element {
    let deck = common_deck.read();

    // Don't render anything if the deck is empty
    if *deck == Default::default() {
        return rsx! {};
    };

    let db = db.read();
    let mut warnings = vec![];

    if deck_check {
        warnings.extend(deck.validate(&db, allow_unreleased, *card_lang.read()));
    }

    // warn on missing proxies
    if proxy_check && has_missing_proxies(&deck, &db, *card_lang.read()) {
        match *card_lang.read() {
            CardLanguage::Japanese => warnings.push("Missing Japanese proxies.".into()),
            CardLanguage::English => warnings.push("Missing English proxies.".into()),
        }
    }

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
    }
}
