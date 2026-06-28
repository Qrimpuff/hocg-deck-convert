use std::{cmp::Reverse, time::Duration};

use dioxus::{prelude::*, web::WebEventExt};
use dioxus_sdk_time::use_debounce;
use gloo::utils::window;
use hocg_fan_sim_assets_model::{CardIllustration, CardReference, CardsDatabase};
use itertools::Itertools;
use jiff::Timestamp;
use js_sys::Date;
use rexie::{ObjectStore, Rexie, TransactionMode};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

use crate::{
    AUTO_SAVE_DECK, CURRENT_PAGE, CardLanguage, EDIT_DECK, IMPORT_FORMAT, PREVIEW_IMAGE_OPTIONS,
    Page, SHOW_PRICE, VERSION, download_file,
    sources::{CommonCard, CommonDeck, DeckLike, DeckOrPile, DeckType, ImageOptions, PileOfCards},
    tracker::{EventType, TrackEvent, track_event, track_internal_url},
};

const SAVE_DB_NAME: &str = "hocg-deck-convert";
const SAVE_STORE_NAME: &str = "saved_decks";
const AUTO_SAVE_KEY: &str = "hocg-deck-convert.auto_saved_deck";
const AUTO_SAVE_DEBOUNCE_MS: u64 = 500;

enum SavedResult {
    Ok(SaveData),
    Err { id: String, error: String },
}

impl SavedResult {
    fn id(&self) -> &str {
        match self {
            SavedResult::Ok(data) => &data.id,
            SavedResult::Err { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SaveData {
    id: String,
    name: String,
    app_version: String,
    saved_at: String,
    deck: SaveDeckOrPile,
}

impl SaveData {
    fn new(deck: SaveDeckOrPile) -> Self {
        let name = deck
            .name()
            .cloned()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| match deck {
                SaveDeckOrPile::Deck(_) => format!("Saved deck {}", Timestamp::now()),
                SaveDeckOrPile::Pile(_) => format!("Saved pile {}", Timestamp::now()),
            });
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            name,
            app_version: VERSION.into(),
            saved_at: Timestamp::now().to_string(),
            deck,
        }
    }

    fn from_deck_or_pile(deck: DeckOrPile, db: &CardsDatabase) -> Self {
        Self::new(SaveDeckOrPile::from_deck_or_pile(&deck, db))
    }

    fn to_deck_or_pile(&self, db: &CardsDatabase) -> DeckOrPile {
        self.deck.to_deck_or_pile(db)
    }

    fn image_path(&self, db: &CardsDatabase) -> String {
        let card_lang = CardLanguage::Japanese;
        let image_options = ImageOptions::card_details();
        match &self.deck {
            SaveDeckOrPile::Deck(save_deck) => save_deck
                .oshi
                .as_ref()
                .and_then(|save_card| save_card.to_common_card(db))
                .and_then(|card| card.image_path(db, card_lang, image_options))
                .unwrap_or("/hocg-deck-convert/assets/cheer-back.webp".to_string()),
            SaveDeckOrPile::Pile(save_pile) => save_pile
                .cards
                .first()
                .as_ref()
                .and_then(|save_card| save_card.to_common_card(db))
                .and_then(|card| card.image_path(db, card_lang, image_options))
                .unwrap_or("/hocg-deck-convert/assets/card-back.webp".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveDeckOrPile {
    Deck(SaveDeck),
    Pile(SavePileOfCards),
}

impl SaveDeckOrPile {
    fn name(&self) -> Option<&String> {
        match self {
            SaveDeckOrPile::Deck(save_deck) => save_deck.name.as_ref(),
            SaveDeckOrPile::Pile(save_pile) => save_pile.name.as_ref(),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            SaveDeckOrPile::Deck(_) => "deck",
            SaveDeckOrPile::Pile(_) => "pile",
        }
    }

    fn from_deck_or_pile(deck_or_pile: &DeckOrPile, db: &CardsDatabase) -> Self {
        match deck_or_pile {
            DeckOrPile::Deck(deck) => SaveDeckOrPile::Deck(SaveDeck::from_deck(deck, db)),
            DeckOrPile::Pile(pile) => SaveDeckOrPile::Pile(SavePileOfCards::from_pile(pile, db)),
        }
    }

    pub fn to_deck_or_pile(&self, db: &CardsDatabase) -> DeckOrPile {
        match self {
            SaveDeckOrPile::Deck(save_deck) => DeckOrPile::Deck(save_deck.to_deck(db)),
            SaveDeckOrPile::Pile(save_pile) => DeckOrPile::Pile(save_pile.to_pile_of_cards(db)),
        }
    }

    fn file_name(&self) -> String {
        let name = self
            .name()
            .cloned()
            .filter(|s| s.is_ascii())
            .unwrap_or_else(|| match self {
                SaveDeckOrPile::Deck(_) => format!("Saved deck {}", Timestamp::now()),
                SaveDeckOrPile::Pile(_) => format!("Saved pile {}", Timestamp::now()),
            });

        let name = name
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' => c,
                _ => '_',
            })
            .fold(String::new(), |mut acc, ch| {
                if ch != '_' || !acc.ends_with('_') {
                    acc.push(ch);
                }
                acc
            });

        match self {
            SaveDeckOrPile::Deck(_) => format!("{}.saved_deck.json", name),
            SaveDeckOrPile::Pile(_) => format!("{}.saved_pile.json", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SaveDeck {
    pub name: Option<String>,
    pub oshi: Option<SaveCard>,
    pub main_deck: Vec<SaveCard>,
    pub cheer_deck: Vec<SaveCard>,
}

impl SaveDeck {
    fn from_deck(deck: &CommonDeck, db: &CardsDatabase) -> Self {
        Self {
            name: deck.name.clone(),
            oshi: deck
                .oshi
                .as_ref()
                .and_then(|card| SaveCard::from_card(card, db)),
            main_deck: deck
                .main_deck
                .iter()
                .filter_map(|card| SaveCard::from_card(card, db))
                .collect(),
            cheer_deck: deck
                .cheer_deck
                .iter()
                .filter_map(|card| SaveCard::from_card(card, db))
                .collect(),
        }
    }

    fn to_deck(&self, db: &CardsDatabase) -> CommonDeck {
        CommonDeck {
            name: self.name.clone(),
            oshi: self
                .oshi
                .as_ref()
                .and_then(|save_card| save_card.to_common_card(db)),
            main_deck: self
                .main_deck
                .iter()
                .filter_map(|save_card| save_card.to_common_card(db))
                .collect(),
            cheer_deck: self
                .cheer_deck
                .iter()
                .filter_map(|save_card| save_card.to_common_card(db))
                .collect(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SavePileOfCards {
    pub name: Option<String>,
    pub cards: Vec<SaveCard>,
}

impl SavePileOfCards {
    fn from_pile(pile: &PileOfCards, db: &CardsDatabase) -> Self {
        let cards = pile
            .cards
            .iter()
            .filter_map(|card| SaveCard::from_card(card, db))
            .collect();
        Self {
            name: pile.name.clone(),
            cards,
        }
    }

    fn to_pile_of_cards(&self, db: &CardsDatabase) -> PileOfCards {
        let cards = self
            .cards
            .iter()
            .filter_map(|save_card| save_card.to_common_card(db))
            .collect();
        PileOfCards {
            name: self.name.clone(),
            cards,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SaveCard {
    pub card: CardReference,
    pub amount: u32,
}

impl SaveCard {
    fn from_card(card: &CommonCard, db: &CardsDatabase) -> Option<Self> {
        Some(Self {
            card: CardIllustration::to_card_ref(card.card_illustration(db)?)?,
            amount: card.amount,
        })
    }

    fn to_common_card(&self, db: &CardsDatabase) -> Option<CommonCard> {
        Some(CommonCard::from_card_illustration(
            self.card.find_in(db)?,
            self.amount,
            db,
        ))
    }
}

fn format_datetime(value: &str) -> String {
    let date = Date::new(&JsValue::from_str(value));
    if date.get_time().is_nan() {
        return value.to_string();
    }

    let year = date.get_full_year();
    let month = date.get_month() + 1;
    let day = date.get_date();
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    let seconds = date.get_seconds();

    let timezone_offset_minutes = -(date.get_timezone_offset() as i32);
    let timezone_sign = if timezone_offset_minutes >= 0 {
        '+'
    } else {
        '-'
    };
    let timezone_hours = timezone_offset_minutes.abs() / 60;
    let timezone_minutes = timezone_offset_minutes.abs() % 60;

    format!(
        "{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}{timezone_sign}{timezone_hours:02}:{timezone_minutes:02}"
    )
}

fn auto_save_deck(save: &SaveData) -> Option<()> {
    // save to local storage
    let ls = window().local_storage().unwrap().unwrap();
    let json = serde_json::to_string(&save).unwrap();
    ls.set_item(AUTO_SAVE_KEY, &json).unwrap();
    Some(())
}

fn delete_auto_saved_deck() -> Option<()> {
    let ls = window().local_storage().unwrap().unwrap();
    ls.remove_item(AUTO_SAVE_KEY).unwrap();
    Some(())
}

fn load_auto_saved_deck() -> Option<SaveData> {
    let ls = window().local_storage().ok()??;
    let json = ls.get_item(AUTO_SAVE_KEY).ok()??;
    let save_data: SaveData = serde_json::from_str(&json).ok()?;
    Some(save_data)
}

async fn open_save_db() -> Result<Rexie, String> {
    Rexie::builder(SAVE_DB_NAME)
        .version(1)
        .add_object_store(ObjectStore::new(SAVE_STORE_NAME).key_path("id"))
        .build()
        .await
        .map_err(|err| format!("Could not open database: {err}"))
}

async fn list_saved_decks() -> Result<Vec<SavedResult>, String> {
    let db = open_save_db().await?;
    let transaction = db
        .transaction(&[SAVE_STORE_NAME], TransactionMode::ReadOnly)
        .map_err(|err| format!("Could not open database transaction: {err}"))?;
    let store = transaction
        .store(SAVE_STORE_NAME)
        .map_err(|err| format!("Could not open save store: {err}"))?;
    let values = store
        .scan(None, None, None, None)
        .await
        .map_err(|err| format!("Could not read saved decks: {err}"))?;
    transaction
        .done()
        .await
        .map_err(|err| format!("Database read transaction failed: {err}"))?;

    let mut values = values
        .into_iter()
        .map(|(key, value)| {
            from_value::<SaveData>(value)
                .map(SavedResult::Ok)
                .unwrap_or_else(|err| SavedResult::Err {
                    id: key.as_string().unwrap_or_default(),
                    error: format!("Could not decode saved deck: {err}"),
                })
        })
        .collect_vec();
    // sort by save time uuid v7
    values.sort_by_key(|value| Reverse(value.id().to_string()));
    Ok(values)
}

async fn save_deck(save: &SaveData) -> Result<(), String> {
    let db = open_save_db().await?;
    let transaction = db
        .transaction(&[SAVE_STORE_NAME], TransactionMode::ReadWrite)
        .map_err(|err| format!("Could not open database transaction: {err}"))?;
    let store = transaction
        .store(SAVE_STORE_NAME)
        .map_err(|err| format!("Could not open save store: {err}"))?;
    let value = to_value(save).map_err(|err| format!("Could not encode saved deck: {err}"))?;
    store
        .put(&value, None)
        .await
        .map_err(|err| format!("Could not save deck: {err}"))?;
    transaction
        .done()
        .await
        .map_err(|err| format!("Database write transaction failed: {err}"))?;
    Ok(())
}

async fn delete_saved_deck(id: &str) -> Result<(), String> {
    let db = open_save_db().await?;
    let transaction = db
        .transaction(&[SAVE_STORE_NAME], TransactionMode::ReadWrite)
        .map_err(|err| format!("Could not open database transaction: {err}"))?;
    let store = transaction
        .store(SAVE_STORE_NAME)
        .map_err(|err| format!("Could not open save store: {err}"))?;
    store
        .delete(id.into())
        .await
        .map_err(|err| format!("Could not delete saved deck: {err}"))?;
    transaction
        .done()
        .await
        .map_err(|err| format!("Database delete transaction failed: {err}"))?;
    Ok(())
}

fn scroll_to_top(container: &mut web_sys::Element) {
    container.set_scroll_top(0);
}

#[derive(Serialize)]
struct SaveLoadEventData {
    action: &'static str,
    item_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl TrackEvent for SaveLoadEventData {}

#[component]
pub fn SaveLoadPage(mut common_deck: Signal<DeckOrPile>, db: Signal<CardsDatabase>) -> Element {
    let mut deck_error = use_signal(String::new);
    let mut is_error_from_file = use_signal(|| false);
    let deck_success = use_signal(String::new);
    let mut is_loading = use_signal(|| true);
    let mut saved_decks = use_signal(Vec::<SavedResult>::new);
    let mut auto_save = use_signal(|| None::<SaveData>);
    let pending_overwrite = use_signal(|| None::<String>);
    let pending_delete = use_signal(|| None::<String>);
    let mut container_ref = use_signal(|| None::<web_sys::Element>);

    use_effect(move || {
        spawn(async move {
            match list_saved_decks().await {
                Ok(saves) => {
                    saved_decks.set(saves);
                }
                Err(err) => {
                    *deck_error.write() = err;
                }
            }
            is_loading.set(false);
        });
    });

    // Create a debounce that waits 500ms after the last edit before executing
    let mut debounced_save =
        use_debounce(Duration::from_millis(AUTO_SAVE_DEBOUNCE_MS), move |()| {
            if let Some(auto_save) = auto_save.read().as_ref() {
                auto_save_deck(auto_save);
            }
        });
    use_effect(move || {
        if let Some(deck) = AUTO_SAVE_DECK.read().as_ref()
            && let save = SaveData::from_deck_or_pile(deck.clone(), &db.read())
            && !save.to_deck_or_pile(&db.read()).is_empty()
        {
            *auto_save.write() = Some(save);
            debounced_save.action(());
        }
    });
    // first auto-save load
    use_effect(move || {
        let save = load_auto_saved_deck();
        *AUTO_SAVE_DECK.write() = save
            .as_ref()
            .map(|save_data| save_data.to_deck_or_pile(&db.read()));
        *auto_save.write() = save;
    });

    let save_to_browser = move |_| {
        let mut deck_error = deck_error;
        let mut deck_success = deck_success;
        let mut pending_overwrite = pending_overwrite;
        let mut pending_delete = pending_delete;
        let mut saved_decks = saved_decks;
        let mut container_ref = container_ref;
        let save = SaveData::from_deck_or_pile(common_deck.read().clone(), &db.read());
        spawn(async move {
            *deck_error.write() = String::new();
            is_error_from_file.set(false);
            *deck_success.write() = String::new();
            pending_overwrite.set(None);
            pending_delete.set(None);
            match save_deck(&save).await {
                Ok(_) => {
                    saved_decks.insert(0, SavedResult::Ok(save.clone()));
                    if let Some(container) = container_ref.write().as_mut() {
                        scroll_to_top(container);
                    }
                    *deck_success.write() = format!("Saved '{}'.", save.name);
                    track_event(
                        EventType::SaveLoad,
                        SaveLoadEventData {
                            action: "Save deck",
                            item_kind: save.deck.kind(),
                            error: None,
                        },
                    );
                }
                Err(err) => {
                    track_event(
                        EventType::SaveLoad,
                        SaveLoadEventData {
                            action: "Save deck",
                            item_kind: save.deck.kind(),
                            error: Some(err.clone()),
                        },
                    );
                    *deck_error.write() = err;
                }
            }
        });
    };

    let save_from_file = move |event: Event<FormData>| {
        let mut deck_error = deck_error;
        let mut deck_success = deck_success;
        let mut pending_overwrite = pending_overwrite;
        let mut pending_delete = pending_delete;
        let mut saved_decks = saved_decks;
        let mut container_ref = container_ref;

        async move {
            *deck_error.write() = String::new();
            is_error_from_file.set(false);
            *deck_success.write() = String::new();
            pending_overwrite.set(None);
            pending_delete.set(None);

            let files = event.files();
            for file in &files {
                match file.read_bytes().await {
                    Ok(contents) => match serde_json::from_slice::<SaveDeckOrPile>(&contents) {
                        Ok(save_deck_or_pile) => {
                            let save = SaveData::new(save_deck_or_pile);
                            match save_deck(&save).await {
                                Ok(_) => {
                                    saved_decks.insert(0, SavedResult::Ok(save.clone()));
                                    if let Some(container) = container_ref.write().as_mut() {
                                        scroll_to_top(container);
                                    }
                                    *deck_success.write() =
                                        format!("Saved '{}' from file.", save.name);
                                    track_event(
                                        EventType::SaveLoad,
                                        SaveLoadEventData {
                                            action: "Add from file",
                                            item_kind: save.deck.kind(),
                                            error: None,
                                        },
                                    );
                                }
                                Err(err) => {
                                    track_event(
                                        EventType::SaveLoad,
                                        SaveLoadEventData {
                                            action: "Add from file",
                                            item_kind: save.deck.kind(),
                                            error: Some(err.clone()),
                                        },
                                    );
                                    *deck_error.write() = err;
                                    is_error_from_file.set(true);
                                }
                            }
                        }
                        Err(err) => {
                            track_event(
                                EventType::SaveLoad,
                                SaveLoadEventData {
                                    action: "Add from file",
                                    item_kind: "unknown",
                                    error: Some(err.to_string()),
                                },
                            );
                            *deck_error.write() = format!("Could not decode save file: {err}");
                            is_error_from_file.set(true);
                        }
                    },
                    Err(err) => {
                        track_event(
                            EventType::SaveLoad,
                            SaveLoadEventData {
                                action: "Add from file",
                                item_kind: "unknown",
                                error: Some(err.to_string()),
                            },
                        );
                        *deck_error.write() = format!("Could not read file: {err}");
                        is_error_from_file.set(true);
                    }
                }
            }
        }
    };

    let keep_auto_save = move |_| {
        let mut deck_error = deck_error;
        let mut deck_success = deck_success;
        let mut pending_overwrite = pending_overwrite;
        let mut pending_delete = pending_delete;
        let mut saved_decks = saved_decks;
        let mut container_ref = container_ref;

        spawn(async move {
            *deck_error.write() = String::new();
            is_error_from_file.set(false);
            *deck_success.write() = String::new();
            pending_overwrite.set(None);
            pending_delete.set(None);

            let save = auto_save.write().take();
            if let Some(save) = save {
                match save_deck(&save).await {
                    Ok(_) => {
                        saved_decks.insert(0, SavedResult::Ok(save.clone()));
                        if let Some(container) = container_ref.write().as_mut() {
                            scroll_to_top(container);
                        }
                        *deck_success.write() = format!("Kept '{}' from auto-save.", save.name);
                        track_event(
                            EventType::SaveLoad,
                            SaveLoadEventData {
                                action: "Keep auto-save",
                                item_kind: save.deck.kind(),
                                error: None,
                            },
                        );
                        delete_auto_saved_deck();
                    }
                    Err(err) => {
                        track_event(
                            EventType::SaveLoad,
                            SaveLoadEventData {
                                action: "Keep auto-save",
                                item_kind: save.deck.kind(),
                                error: Some(err.clone()),
                            },
                        );
                        *deck_error.write() = err;
                        *auto_save.write() = Some(save);
                    }
                }
            }
        });
    };

    rsx! {
        div { class: "content",
            p { "Save the current deck or pile in this browser, then load it later on the same device." }
            if *is_loading.read() {
                p { class: "has-text-grey", "Loading saved decks..." }
            }
        }

        div { class: "field is-grouped is-grouped-multiline is-justify-content-center",
            div { class: "control",
                button {
                    class: "button is-link",
                    r#type: "button",
                    onclick: save_to_browser,
                    disabled: *is_loading.read() || common_deck.read().is_empty(),
                    span { class: "icon",
                        i { class: "fa-solid fa-floppy-disk" }
                    }
                    span { "Save current deck" }
                }
            }
            div { class: "control",
                div { class: "file",
                    label { class: "file-label",
                        input {
                            r#type: "file",
                            class: "file-input",
                            accept: ".json",
                            onchange: save_from_file,
                            disabled: *is_loading.read(),
                        }
                        span { class: "file-cta",
                            span { class: "file-icon",
                                i { class: "fa-solid fa-upload" }
                            }
                            span { class: "file-label", "Add deck from file..." }
                        }
                    }
                }
            }
        }

        // p { class: "help is-success content", "{deck_success}" }
        p { class: "help is-danger content", "{deck_error}" }
        if is_error_from_file.read().to_owned() {
            div { class: "notification is-warning",
                "If you are uncertain about the kind of file you have, try importing using "
                a {
                    href: "#",
                    role: "button",
                    onclick: move |evt| {
                        evt.prevent_default();
                        *IMPORT_FORMAT.write() = Some(DeckType::Unknown);
                        *CURRENT_PAGE.write() = Page::Import;
                        *EDIT_DECK.write() = false;
                        *SHOW_PRICE.write() = false;
                        *PREVIEW_IMAGE_OPTIONS.write() = ImageOptions::holodelta();
                        track_internal_url("Import - I don't know...");
                    },
                    "\"I don't know...\""
                }
                " instead. "
            }
        }

        // auto-saved deck
        if let Some(save) = auto_save.read().as_ref() {
            div {
                p { class: "mt-3 mb-2",
                    "Auto-saved "
                    match save.deck {
                        SaveDeckOrPile::Deck(_) => "deck",
                        SaveDeckOrPile::Pile(_) => "pile of cards",
                    }
                }
            }
            div { class: "cell", style: "transition: background-color 0.2s;",
                article { class: "message is-small",
                    div { class: "message-body",
                        div { class: "is-flex is-justify-content-end is-align-items-center is-flex-wrap-wrap is-gap-2",
                            div {
                                class: "is-flex is-align-items-center is-gap-2 is-flex-grow-1",
                                style: "flex: 1 1 14.75rem; min-width: 0;",
                                div { class: "is-flex is-align-items-center",
                                    img {
                                        style: "height: 42px; width: 30px; min-width: 30px;",
                                        width: "400",
                                        height: "560",
                                        border_radius: "4.9% / 3.5%",
                                        src: "{save.image_path(&db.read())}",
                                        "onerror": if matches!(save.deck, SaveDeckOrPile::Deck(_)) { "this.src='/hocg-deck-convert/assets/cheer-back.webp'" } else { "this.src='/hocg-deck-convert/assets/card-back.webp'" },
                                    }
                                }
                                div {
                                    class: "is-flex-grow-1",
                                    style: "min-width: 0;",
                                    p { class: "has-text-weight-semibold", "{save.name}" }
                                    p { class: "is-size-7 has-text-grey",
                                        "Saved at {format_datetime(&save.saved_at)}"
                                    }
                                }
                            }
                            div {
                                class: "buttons are-small is-flex-wrap-nowrap",
                                style: "flex: 0 0 auto; margin-bottom: 0; gap: 0.25rem;",
                                button {
                                    class: "button is-link",
                                    r#type: "button",
                                    title: "Load this deck",
                                    aria_label: format!("Load saved deck '{}'", save.name),
                                    onclick: {
                                        let save = save.clone();
                                        let mut common_deck = common_deck;
                                        let mut deck_error = deck_error;
                                        let mut deck_success = deck_success;
                                        let mut pending_overwrite = pending_overwrite;
                                        let mut pending_delete = pending_delete;
                                        move |_| {
                                            *deck_error.write() = String::new();
                                            is_error_from_file.set(false);
                                            *deck_success.write() = String::new();
                                            pending_overwrite.set(None);
                                            pending_delete.set(None);
                                            *common_deck.write() = save.to_deck_or_pile(&db.read());
                                            *deck_success.write() = format!("Loaded '{}'.", save.name);
                                            track_event(
                                                EventType::SaveLoad,
                                                SaveLoadEventData {
                                                    action: "Load auto-saved deck",
                                                    item_kind: save.deck.kind(),
                                                    error: None,
                                                },
                                            );
                                        }
                                    },
                                    disabled: *is_loading.read(),
                                    span { class: "icon",
                                        i { class: "fa-solid fa-arrow-right-from-bracket" }
                                    }
                                    span { "Load" }
                                }
                                button {
                                    class: "button",
                                    r#type: "button",
                                    title: "Keep this deck",
                                    aria_label: format!("Keep this deck '{}'", save.name),
                                    onclick: keep_auto_save,
                                    disabled: *is_loading.read(),
                                    span { class: "icon",
                                        i { class: "fa-solid fa-floppy-disk" }
                                    }
                                    span { "Keep" }
                                }
                            }
                        }
                    }
                }
            }
        }

        // saved decks list
        if !*is_loading.read() && saved_decks.read().is_empty() {
            div { class: "mt-5 notification",
                p { "No saved decks yet. Save the current deck to add it to the list." }
            }
        } else if !saved_decks.read().is_empty() {
            div {
                p { class: "mt-3 mb-2",
                    "Saved decks"
                    if !saved_decks.read().is_empty() {
                        " ({saved_decks.read().len()})"
                    }
                }
            }
            div {
                class: "fixed-grid has-1-cols",
                style: "max-height: 65vh; overflow: scroll;",
                onmounted: move |elem| {
                    *container_ref.write() = Some(elem.as_web_event());
                },
                div { class: "grid",
                    for save in saved_decks.read().iter() {
                        div {
                            class: "cell",
                            style: "transition: background-color 0.2s;",
                            if let SavedResult::Err { id, error } = &save {
                                article { class: "message is-small is-danger",
                                    div { class: "message-body",
                                        div { class: "is-flex is-justify-content-end is-align-items-center is-flex-wrap-wrap is-gap-2",
                                            div {
                                                class: "is-flex is-align-items-center is-gap-2 is-flex-grow-1",
                                                style: "flex: 1 1 14.75rem; min-width: 0;",
                                                div {
                                                    div { class: "is-flex is-align-items-center is-gap-2",
                                                        span { class: "icon",
                                                            i { class: "fa-solid fa-lg fa-triangle-exclamation" }
                                                        }
                                                        p { "Error loading this saved deck: " }
                                                    }
                                                    p { class: "is-size-7 has-text-grey",
                                                        "{error}"
                                                    }
                                                }
                                            }
                                            div {
                                                class: "buttons are-small is-flex-wrap-nowrap",
                                                style: "flex: 0 0 auto; margin-bottom: 0; gap: 0.25rem;",
                                                button {
                                                    class: "button is-danger",
                                                    r#type: "button",
                                                    title: "Delete this entry",
                                                    aria_label: "Delete this error entry",
                                                    onclick: {
                                                        let id = id.clone();
                                                        let deck_error = deck_error;
                                                        let saved_decks = saved_decks;
                                                        move |_| {
                                                            let id = id.clone();
                                                            let mut deck_error = deck_error;
                                                            let mut deck_success = deck_success;
                                                            let mut saved_decks = saved_decks;
                                                            spawn(async move {
                                                                *deck_error.write() = String::new();
                                                                is_error_from_file.set(false);
                                                                *deck_success.write() = String::new();
                                                                match delete_saved_deck(&id).await {
                                                                    Ok(_) => {
                                                                        saved_decks.write().retain(|save| { save.id() != id });
                                                                        *deck_success.write() = "Deleted error entry.".to_string();
                                                                        track_event(
                                                                            EventType::SaveLoad,
                                                                            SaveLoadEventData {
                                                                                action: "Delete deck",
                                                                                item_kind: "unknown",
                                                                                error: None,
                                                                            },
                                                                        );
                                                                    }
                                                                    Err(err) => {
                                                                        track_event(
                                                                            EventType::SaveLoad,
                                                                            SaveLoadEventData {
                                                                                action: "Delete deck",
                                                                                item_kind: "unknown",
                                                                                error: Some(err.clone()),
                                                                            },
                                                                        );
                                                                        *deck_error.write() = err;
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    disabled: *is_loading.read(),
                                                    span { class: "icon",
                                                        i { class: "fa-solid fa-trash" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let SavedResult::Ok(save) = &save {
                                article { class: "message is-small",
                                    div { class: "message-body",
                                        div { class: "is-flex is-justify-content-end is-align-items-center is-flex-wrap-wrap is-gap-2",
                                            div {
                                                class: "is-flex is-align-items-center is-gap-2 is-flex-grow-1",
                                                style: "flex: 1 1 14.75rem; min-width: 0;",
                                                div { class: "is-flex is-align-items-center",
                                                    img {
                                                        style: "height: 42px; width: 30px; min-width: 30px;",
                                                        width: "400",
                                                        height: "560",
                                                        border_radius: "4.9% / 3.5%",
                                                        src: "{save.image_path(&db.read())}",
                                                        "onerror": if matches!(save.deck, SaveDeckOrPile::Deck(_)) { "this.src='/hocg-deck-convert/assets/cheer-back.webp'" } else { "this.src='/hocg-deck-convert/assets/card-back.webp'" },
                                                    }
                                                }
                                                div {
                                                    class: "is-flex-grow-1",
                                                    style: "min-width: 0;",
                                                    p { class: "has-text-weight-semibold",
                                                        "{save.name}"
                                                    }
                                                    if pending_overwrite.read().as_ref() == Some(&save.id) {
                                                        p { class: "is-size-7 has-text-warning",
                                                            "Click save again to confirm."
                                                        }
                                                    } else if pending_delete.read().as_ref() == Some(&save.id) {
                                                        p { class: "is-size-7 has-text-danger",
                                                            "Click delete again to confirm."
                                                        }
                                                    } else {
                                                        p { class: "is-size-7 has-text-grey",
                                                            "Saved at {format_datetime(&save.saved_at)}"
                                                        }
                                                    }
                                                }
                                            }
                                            div {
                                                class: "buttons are-small is-flex-wrap-nowrap",
                                                style: "flex: 0 0 auto; margin-bottom: 0; gap: 0.25rem;",
                                                button {
                                                    class: "button is-link",
                                                    r#type: "button",
                                                    title: "Load this deck",
                                                    aria_label: format!("Load saved deck '{}'", save.name),
                                                    onclick: {
                                                        let save = save.clone();
                                                        let mut common_deck = common_deck;
                                                        let mut deck_error = deck_error;
                                                        let mut deck_success = deck_success;
                                                        let mut pending_overwrite = pending_overwrite;
                                                        let mut pending_delete = pending_delete;
                                                        move |_| {
                                                            *deck_error.write() = String::new();
                                                            is_error_from_file.set(false);
                                                            *deck_success.write() = String::new();
                                                            pending_overwrite.set(None);
                                                            pending_delete.set(None);
                                                            *common_deck.write() = save.to_deck_or_pile(&db.read());
                                                            *deck_success.write() = format!("Loaded '{}'.", save.name);
                                                            track_event(
                                                                EventType::SaveLoad,
                                                                SaveLoadEventData {
                                                                    action: "Load deck",
                                                                    item_kind: save.deck.kind(),
                                                                    error: None,
                                                                },
                                                            );
                                                        }
                                                    },
                                                    disabled: *is_loading.read(),
                                                    span { class: "icon",
                                                        i { class: "fa-solid fa-arrow-right-from-bracket" }
                                                    }
                                                    span { "Load" }
                                                }
                                                button {
                                                    class: if pending_overwrite.read().as_ref() == Some(&save.id) { "button is-success" } else { "button" },
                                                    r#type: "button",
                                                    title: if pending_overwrite.read().as_ref() == Some(&save.id) { "Click again to overwrite this deck" } else { "Overwrite this deck" },
                                                    aria_label: format!("Overwrite saved deck '{}'", save.name),
                                                    onclick: {
                                                        let id = save.id.clone();
                                                        let name = save.name.clone();
                                                        let item_kind = save.deck.kind();
                                                        let mut pending_overwrite = pending_overwrite;
                                                        let mut pending_delete = pending_delete;
                                                        let deck_error = deck_error;
                                                        let saved_decks = saved_decks;
                                                        move |_| {
                                                            let id = id.clone();
                                                            let name = name.clone();
                                                            let item_kind = item_kind;
                                                            pending_delete.set(None);
                                                            if pending_overwrite.read().as_ref() != Some(&id) {
                                                                pending_overwrite.set(Some(id.clone()));
                                                                return;
                                                            }
                                                            pending_overwrite.set(None);
                                                            let mut deck_error = deck_error;
                                                            let mut deck_success = deck_success;
                                                            let mut saved_decks = saved_decks;
                                                            spawn(async move {
                                                                *deck_error.write() = String::new();
                                                                is_error_from_file.set(false);
                                                                *deck_success.write() = String::new();
                                                                let mut new_save = SaveData::from_deck_or_pile(
                                                                    common_deck.read().clone(),
                                                                    &db.read(),
                                                                );
                                                                new_save.id = id.clone();
                                                                match save_deck(&new_save).await {
                                                                    Ok(_) => {
                                                                        if let Some(save) = saved_decks
                                                                            .write()
                                                                            .iter_mut()
                                                                            .find(|save| save.id() == id)
                                                                        {
                                                                            *save = SavedResult::Ok(new_save.clone());
                                                                        }
                                                                        *deck_success.write() = format!("Overwritten '{}'.", name);
                                                                        track_event(
                                                                            EventType::SaveLoad,
                                                                            SaveLoadEventData {
                                                                                action: "Overwrite deck",
                                                                                item_kind,
                                                                                error: None,
                                                                            },
                                                                        );
                                                                    }
                                                                    Err(err) => {
                                                                        track_event(
                                                                            EventType::SaveLoad,
                                                                            SaveLoadEventData {
                                                                                action: "Overwrite deck",
                                                                                item_kind,
                                                                                error: Some(err.clone()),
                                                                            },
                                                                        );
                                                                        *deck_error.write() = err;
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    disabled: *is_loading.read() || common_deck.read().is_empty(),
                                                    span { class: "icon",
                                                        i { class: "fa-solid fa-floppy-disk" }
                                                    }
                                                    span { "Save" }
                                                }
                                                button {
                                                    class: "button",
                                                    r#type: "button",
                                                    title: "Download this deck",
                                                    aria_label: format!("Download saved deck '{}'", save.name),
                                                    onclick: {
                                                        let save = save.clone();
                                                        let mut deck_error = deck_error;
                                                        let mut deck_success = deck_success;
                                                        let mut pending_overwrite = pending_overwrite;
                                                        let mut pending_delete = pending_delete;
                                                        move |_| {
                                                            *deck_error.write() = String::new();
                                                            is_error_from_file.set(false);
                                                            *deck_success.write() = String::new();
                                                            pending_overwrite.set(None);
                                                            pending_delete.set(None);
                                                            match serde_json::to_vec_pretty(&save.deck) {
                                                                Ok(contents) => {
                                                                    download_file(&save.deck.file_name(), &contents[..]);
                                                                    *deck_success.write() = format!("Downloaded '{}'.", save.name);
                                                                    track_event(
                                                                        EventType::SaveLoad,
                                                                        SaveLoadEventData {
                                                                            action: "Download deck",
                                                                            item_kind: save.deck.kind(),
                                                                            error: None,
                                                                        },
                                                                    );
                                                                }
                                                                Err(err) => {
                                                                    track_event(
                                                                        EventType::SaveLoad,
                                                                        SaveLoadEventData {
                                                                            action: "Download deck",
                                                                            item_kind: save.deck.kind(),
                                                                            error: Some(format!("Could not encode save file: {err}")),
                                                                        },
                                                                    );
                                                                    *deck_error.write() = format!("Could not encode save file: {err}");
                                                                }
                                                            }
                                                        }
                                                    },
                                                    disabled: *is_loading.read(),
                                                    span { class: "icon",
                                                        i { class: "fa-solid fa-download" }
                                                    }
                                                }
                                                button {
                                                    class: if pending_delete.read().as_ref() == Some(&save.id) { "button is-danger" } else { "button" },
                                                    r#type: "button",
                                                    title: if pending_delete.read().as_ref() == Some(&save.id) { "Click again to delete this deck" } else { "Delete this deck" },
                                                    aria_label: format!("Delete saved deck '{}'", save.name),
                                                    onclick: {
                                                        let id = save.id.clone();
                                                        let name = save.name.clone();
                                                        let item_kind = save.deck.kind();
                                                        let mut pending_overwrite = pending_overwrite;
                                                        let mut pending_delete = pending_delete;
                                                        let deck_error = deck_error;
                                                        let saved_decks = saved_decks;
                                                        move |_| {
                                                            let id = id.clone();
                                                            let name = name.clone();
                                                            let item_kind = item_kind;
                                                            pending_overwrite.set(None);
                                                            if pending_delete.read().as_ref() != Some(&id) {
                                                                pending_delete.set(Some(id.clone()));
                                                                return;
                                                            }
                                                            pending_delete.set(None);
                                                            let mut deck_error = deck_error;
                                                            let mut deck_success = deck_success;
                                                            let mut saved_decks = saved_decks;
                                                            spawn(async move {
                                                                *deck_error.write() = String::new();
                                                                is_error_from_file.set(false);
                                                                *deck_success.write() = String::new();
                                                                match delete_saved_deck(&id).await {
                                                                    Ok(_) => {
                                                                        saved_decks.write().retain(|save| { save.id() != id });
                                                                        *deck_success.write() = format!("Deleted '{}'.", name);
                                                                        track_event(
                                                                            EventType::SaveLoad,
                                                                            SaveLoadEventData {
                                                                                action: "Delete deck",
                                                                                item_kind,
                                                                                error: None,
                                                                            },
                                                                        );
                                                                    }
                                                                    Err(err) => {
                                                                        track_event(
                                                                            EventType::SaveLoad,
                                                                            SaveLoadEventData {
                                                                                action: "Delete deck",
                                                                                item_kind,
                                                                                error: Some(err.clone()),
                                                                            },
                                                                        );
                                                                        *deck_error.write() = err;
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    disabled: *is_loading.read(),
                                                    span { class: if pending_delete.read().as_ref() == Some(&save.id) { "icon " } else { "icon has-text-danger" },
                                                        i { class: "fa-solid fa-trash" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
