use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsDatabase;

use crate::{CardLanguage, sources::CommonDeck};

#[component]
pub fn DeckValidation(
    deck_check: bool,
    proxy_check: bool,
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
        warnings.extend(deck.validate(&db));
    }

    // warn on missing english proxy
    if proxy_check
        && *card_lang.read() == CardLanguage::English
        && deck
            .all_cards()
            .any(|c| c.image_path(&db, *card_lang.read(), true, false).is_none())
    {
        warnings.push("Missing english proxy.".into());
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
