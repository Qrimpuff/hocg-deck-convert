use dioxus::prelude::*;

use crate::components::card_search::CardSearch;

use super::{CardsDatabase, CommonDeck};

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
    is_edit: Signal<bool>,
) -> Element {
    // sort the deck when entering the edit page
    use_effect(move || common_deck.write().sort(&db.read()));

    rsx! {
        CardSearch { db, common_deck, is_edit }
    }
}
