use dioxus::prelude::*;

use crate::{
    components::card_search::CardSearch,
    sources::{DeckLike, DeckOrPile},
};

use super::CardsDatabase;

#[component]
pub fn Import(
    mut common_deck: Signal<DeckOrPile>,
    db: Signal<CardsDatabase>,
    is_edit: Signal<bool>,
) -> Element {
    // sort the deck when entering the edit page
    use_effect(move || common_deck.write().sort(&db.read()));

    rsx! {
        CardSearch { db, common_deck, is_edit }
    }
}
