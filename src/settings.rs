use dioxus::{document::document, prelude::*};
use gloo::utils::window;
use serde::{Deserialize, Serialize};

const SETTINGS_KEY: &str = "hocg_settings";
const CARD_IMG_PRESETS: [(&str, u32); 4] = [("S", 80), ("M", 110), ("L", 140), ("XL", 180)];
const CARD_IMG_MIN_WIDTH: u32 = 60;
const CARD_IMG_MAX_WIDTH: u32 = 200;

pub static SETTINGS: GlobalSignal<Settings> = Signal::global(Settings::load);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub card_img_width: u32,
    pub standalone_mode: Option<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            card_img_width: 80,
            standalone_mode: None,
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

pub fn set_card_img_width(width: u32) {
    let width = width.clamp(CARD_IMG_MIN_WIDTH, CARD_IMG_MAX_WIDTH);
    let mut settings = SETTINGS.write();
    settings.card_img_width = width;
    settings.save();
}

pub fn set_standalone_mode(enabled: bool) {
    {
        let mut settings = SETTINGS.write();
        settings.standalone_mode = Some(enabled);
        settings.save();
    }
    if enabled {
        document().eval("window.hocgStartStandaloneDownload()".to_string());
    }
}

fn is_installed_app() -> bool {
    js_sys::eval("window.matchMedia('(display-mode: standalone)').matches || !!window.navigator.standalone")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[component]
pub fn SettingsPage() -> Element {
    let width = SETTINGS.read().card_img_width;
    let standalone_mode = SETTINGS.read().standalone_mode.unwrap_or_else(is_installed_app);

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
                            onclick: move |_| set_card_img_width(preset),
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
                }
                span { class: "is-flex-shrink-0", style: "min-width: 4em; text-align: right", "{width}px" }
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
            p { class: "help", "Downloads card images so the app works offline on mobile and desktop. Uncheck to opt out." }
        }
    }
}
