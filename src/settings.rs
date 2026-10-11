use crate::{
    app_state,
    tracker::{EventType, TrackEvent, track_event},
};
use dioxus::{document::document, prelude::*};
use gloo::utils::window;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Cache;

const SETTINGS_KEY: &str = "hocg_settings";
const CARD_IMG_PRESETS: [(&str, u32); 4] = [("S", 80), ("M", 110), ("L", 140), ("XL", 180)];
const CARD_IMG_MIN_WIDTH: u32 = 60;
const CARD_IMG_MAX_WIDTH: u32 = 200;
const CARD_CACHE_PREFIX: &str = "hocg-deck-convert-cards";
const PRICE_CACHE_PREFIX: &str = "hocg-deck-convert-prices";
const APP_CACHE_PREFIX: &str = "hocg-deck-convert-";

pub static SETTINGS: GlobalSignal<Settings> = Signal::global(Settings::load);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub card_img_width: u32,
    pub standalone_mode: Option<bool>,
    pub keep_app_state: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            card_img_width: 80,
            standalone_mode: None,
            keep_app_state: true,
        }
    }
}

impl Settings {
    fn load() -> Self {
        window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|ls| ls.get_item(SETTINGS_KEY).ok().flatten())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        let Some(ls) = window().local_storage().ok().flatten() else {
            return;
        };
        if let Ok(json) = serde_json::to_string(self) {
            let _ = ls.set_item(SETTINGS_KEY, &json);
        }
    }
}

#[derive(Serialize)]
struct SettingsEventData {
    setting: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_action: Option<&'static str>,
}
impl TrackEvent for SettingsEventData {}

fn track_setting(setting: &'static str, enabled: Option<bool>, cache_action: Option<&'static str>) {
    track_event(
        EventType::Settings(setting.into()),
        SettingsEventData {
            setting,
            enabled,
            cache_action,
        },
    );
}

pub fn set_card_img_width(width: u32) {
    let width = width.clamp(CARD_IMG_MIN_WIDTH, CARD_IMG_MAX_WIDTH);
    let mut settings = SETTINGS.write();
    settings.card_img_width = width;
    settings.save();
}

pub fn keep_app_state() -> bool {
    SETTINGS.read().keep_app_state
}

pub fn set_keep_app_state(enabled: bool) {
    track_setting("Keep app state", Some(enabled), None);
    {
        let mut settings = SETTINGS.write();
        settings.keep_app_state = enabled;
        settings.save();
    }
    if enabled {
        app_state::save();
    } else {
        app_state::clear();
    }
}

pub fn set_standalone_mode(enabled: bool) {
    track_setting("Standalone mode", Some(enabled), None);
    {
        let mut settings = SETTINGS.write();
        settings.standalone_mode = Some(enabled);
        settings.save();
    }
    if enabled {
        document().eval("window.hocgStartStandaloneDownload()".to_string());
    }
}

#[derive(Debug, Clone, Copy)]
struct CacheStats {
    card_images: u64,
    price_files: u64,
    app_files: u64,
    usage: Option<u64>,
    quota: Option<u64>,
}

fn js_strings(value: JsValue) -> Vec<String> {
    let array: js_sys::Array = value.unchecked_into();
    array.iter().filter_map(|v| v.as_string()).collect()
}

async fn clear_card_cache() {
    let Ok(storage) = window().caches() else {
        return;
    };
    let Ok(names) = JsFuture::from(storage.keys()).await else {
        return;
    };
    for name in js_strings(names) {
        if name.starts_with(CARD_CACHE_PREFIX) {
            let _ = JsFuture::from(storage.delete(&name)).await;
        }
    }
}

async fn load_cache_stats() -> Option<CacheStats> {
    let storage = window().caches().ok()?;
    let names = js_strings(JsFuture::from(storage.keys()).await.ok()?);
    let mut card_images = 0;
    let mut price_files = 0;
    let mut app_files = 0;
    for name in names {
        let cache: Cache = JsFuture::from(storage.open(&name))
            .await
            .ok()?
            .unchecked_into();
        let count = JsFuture::from(cache.keys())
            .await
            .ok()?
            .unchecked_into::<js_sys::Array>()
            .length() as u64;
        if name.starts_with(CARD_CACHE_PREFIX) {
            card_images += count;
        } else if name.starts_with(PRICE_CACHE_PREFIX) {
            price_files += count;
        } else if name.starts_with(APP_CACHE_PREFIX) {
            app_files += count;
        }
    }
    let (usage, quota) = storage_estimate().await;
    Some(CacheStats {
        card_images,
        price_files,
        app_files,
        usage,
        quota,
    })
}

async fn storage_estimate() -> (Option<u64>, Option<u64>) {
    let nav: JsValue = window().navigator().into();
    let Ok(manager) = js_sys::Reflect::get(&nav, &"storage".into()) else {
        return (None, None);
    };
    let Ok(estimate_fn) = js_sys::Reflect::get(&manager, &"estimate".into()) else {
        return (None, None);
    };
    let Ok(promise) = estimate_fn
        .unchecked_into::<js_sys::Function>()
        .call0(&manager)
    else {
        return (None, None);
    };
    let Ok(estimate) = JsFuture::from(js_sys::Promise::from(promise)).await else {
        return (None, None);
    };
    let read = |key: &str| {
        js_sys::Reflect::get(&estimate, &key.into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as u64)
    };
    (read("usage"), read("quota"))
}

fn format_bytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

fn is_installed_app() -> bool {
    js_sys::eval(
        "window.matchMedia('(display-mode: standalone)').matches || !!window.navigator.standalone",
    )
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false)
}

#[component]
pub fn SettingsPage() -> Element {
    let width = SETTINGS.read().card_img_width;
    let standalone_mode = SETTINGS
        .read()
        .standalone_mode
        .unwrap_or_else(is_installed_app);
    let keep_app_state = SETTINGS.read().keep_app_state;
    let mut stats = use_resource(load_cache_stats);
    let cache_text = match &*stats.read() {
        None => "Loading...".to_string(),
        Some(None) => "Cache stats are not available.".to_string(),
        Some(Some(s)) => {
            let storage = match (s.usage, s.quota) {
                (Some(usage), Some(quota)) => {
                    format!(
                        " Site storage: {} of {}.",
                        format_bytes(usage),
                        format_bytes(quota)
                    )
                }
                _ => String::new(),
            };
            format!(
                "App files: {}. Card images: {}. Price data: {}.{storage}",
                s.app_files, s.card_images, s.price_files
            )
        }
    };

    rsx! {
        div { class: "field",
            label { class: "label", "Deck preview image size" }
            div { class: "is-flex is-align-items-center",
                div { class: "buttons has-addons mb-0 is-flex-shrink-0",
                    for (label, preset) in CARD_IMG_PRESETS {
                        button {
                            class: "button is-small",
                            class: if width == preset { "is-link is-selected" },
                            r#type: "button",
                            onclick: move |_| {
                                track_setting("Preview size", None, None);
                                set_card_img_width(preset);
                            },
                            "{label}"
                        }
                    }
                }
                input {
                    class: "ml-4 mr-4",
                    style: "flex: 1 1 auto; min-width: 0",
                    r#type: "range",
                    min: "{CARD_IMG_MIN_WIDTH}",
                    max: "{CARD_IMG_MAX_WIDTH}",
                    step: "10",
                    value: "{width}",
                    oninput: move |ev| {
                        if let Ok(w) = ev.value().parse::<u32>() {
                            set_card_img_width(w);
                        }
                    },
                    onchange: move |_| track_setting("Preview size", None, None),
                }
                span {
                    class: "is-flex-shrink-0",
                    style: "min-width: 4em; text-align: right",
                    "{width}px"
                }
            }
        }
        div { class: "field mt-5",
            div { class: "control",
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: standalone_mode,
                        onchange: move |ev| set_standalone_mode(ev.checked()),
                    }
                    " Standalone mode"
                }
            }
            p { class: "help",
                "Downloads card images so the app works offline on mobile and desktop. Uncheck to opt out."
            }
        }
        div { class: "field mt-5",
            div { class: "control",
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: keep_app_state,
                        onchange: move |ev| set_keep_app_state(ev.checked()),
                    }
                    " Keep app state"
                }
            }
            p { class: "help",
                "Remembers the current page, formats, filters and deck, so you can continue after closing the app."
            }
        }
        div { class: "field mt-5",
            label { class: "label", "Cache" }
            p { class: "help", "{cache_text}" }
            div { class: "buttons mt-2",
                button {
                    class: "button is-small",
                    r#type: "button",
                    onclick: move |_| {
                        track_setting("Cache", None, Some("refresh"));
                        stats.restart();
                    },
                    "Refresh"
                }
                button {
                    class: "button is-small has-text-danger",
                    r#type: "button",
                    onclick: move |_| {
                        track_setting("Cache", None, Some("clear"));
                        spawn(async move {
                            clear_card_cache().await;
                            stats.restart();
                        });
                    },
                    "Clear card images"
                }
            }
        }
    }
}
