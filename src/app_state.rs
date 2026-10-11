use dioxus::prelude::*;
use gloo::utils::window;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    COMMON_DECK, CURRENT_PAGE, CardLanguage, EDIT_DECK, EXPORT_FORMAT, FREE_BASIC_CHEERS,
    GLOBAL_RARITY, GLOBAL_RELEASE, IMPORT_FORMAT, PREVIEW_CARD_LANG, PREVIEW_IMAGE_OPTIONS,
    PRICE_SERVICE, Page, SHOW_PRICE,
    components::{
        card_details::CARD_DETAILS_LANG,
        card_search::{FilterRarity, FilterRelease},
    },
    settings::{SETTINGS, keep_app_state},
    sources::{DeckOrPile, DeckType, ImageOptions, price_check::PriceCheckService},
};

const APP_STATE_KEY: &str = "hocg-deck-convert.app_state";
const STATE_PREFIX: &str = "hocg-deck-convert.state.";

fn local_storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

/// Reads the setting without subscribing. Use at startup and during signal init.
fn keep_app_state_untracked() -> bool {
    SETTINGS.peek().keep_app_state
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct AppState {
    page: Page,
    common_deck: DeckOrPile,
    import_format: Option<DeckType>,
    export_format: Option<DeckType>,
    edit_deck: bool,
    show_price: bool,
    preview_card_lang: CardLanguage,
    preview_image_options: ImageOptions,
    card_details_lang: CardLanguage,
    price_service: PriceCheckService,
    global_release: FilterRelease,
    global_rarity: FilterRarity,
    free_basic_cheers: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            page: Page::Import,
            common_deck: DeckOrPile::default(),
            import_format: None,
            export_format: None,
            edit_deck: false,
            show_price: false,
            preview_card_lang: CardLanguage::Japanese,
            preview_image_options: ImageOptions::holodelta(),
            card_details_lang: CardLanguage::English,
            price_service: PriceCheckService::Yuyutei,
            global_release: FilterRelease::All,
            global_rarity: FilterRarity::All,
            free_basic_cheers: false,
        }
    }
}

impl AppState {
    fn current() -> Self {
        Self {
            page: *CURRENT_PAGE.read(),
            common_deck: COMMON_DECK.read().clone(),
            import_format: *IMPORT_FORMAT.read(),
            export_format: *EXPORT_FORMAT.read(),
            edit_deck: *EDIT_DECK.read(),
            show_price: *SHOW_PRICE.read(),
            preview_card_lang: *PREVIEW_CARD_LANG.read(),
            preview_image_options: *PREVIEW_IMAGE_OPTIONS.read(),
            card_details_lang: *CARD_DETAILS_LANG.read(),
            price_service: *PRICE_SERVICE.read(),
            global_release: *GLOBAL_RELEASE.read(),
            global_rarity: GLOBAL_RARITY.read().clone(),
            free_basic_cheers: *FREE_BASIC_CHEERS.read(),
        }
    }

    fn apply(self) {
        *CURRENT_PAGE.write() = self.page;
        *COMMON_DECK.write() = self.common_deck;
        *IMPORT_FORMAT.write() = self.import_format;
        *EXPORT_FORMAT.write() = self.export_format;
        *EDIT_DECK.write() = self.edit_deck;
        *SHOW_PRICE.write() = self.show_price;
        *PREVIEW_CARD_LANG.write() = self.preview_card_lang;
        *PREVIEW_IMAGE_OPTIONS.write() = self.preview_image_options;
        *CARD_DETAILS_LANG.write() = self.card_details_lang;
        *PRICE_SERVICE.write() = self.price_service;
        *GLOBAL_RELEASE.write() = self.global_release;
        *GLOBAL_RARITY.write() = self.global_rarity;
        *FREE_BASIC_CHEERS.write() = self.free_basic_cheers;
    }
}

/// Writes the page-level state on every change, when the setting is enabled.
pub fn save() {
    if !keep_app_state() {
        return;
    }
    let (Some(ls), Ok(json)) = (
        local_storage(),
        serde_json::to_string(&AppState::current()),
    ) else {
        return;
    };
    let _ = ls.set_item(APP_STATE_KEY, &json);
}

/// Restores the page-level state before any component renders.
pub fn restore() {
    if !keep_app_state_untracked() {
        return;
    }
    let state = local_storage()
        .and_then(|ls| ls.get_item(APP_STATE_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str::<AppState>(&json).ok())
        .unwrap_or_default();
    state.apply();
}

fn load_value<T: DeserializeOwned>(key: &str) -> Option<T> {
    if !keep_app_state_untracked() {
        return None;
    }
    let json = local_storage()?
        .get_item(&format!("{STATE_PREFIX}{key}"))
        .ok()??;
    serde_json::from_str(&json).ok()
}

fn store_value<T: Serialize>(key: &str, value: &T) {
    if !keep_app_state() {
        return;
    }
    let (Some(ls), Ok(json)) = (local_storage(), serde_json::to_string(value)) else {
        return;
    };
    let _ = ls.set_item(&format!("{STATE_PREFIX}{key}"), &json);
}

/// Signal saved to localStorage under `key` while keep app state is on. `None` disables persistence.
pub fn use_persisted<T>(key: Option<String>, init: impl FnOnce() -> T) -> Signal<T>
where
    T: Serialize + DeserializeOwned + Clone + 'static,
{
    let restore_key = key.clone();
    let signal = use_signal(move || {
        restore_key
            .as_deref()
            .and_then(load_value::<T>)
            .unwrap_or_else(init)
    });
    use_effect(move || {
        let value = signal.read().clone();
        if let Some(key) = &key {
            store_value(key, &value);
        }
    });
    signal
}
