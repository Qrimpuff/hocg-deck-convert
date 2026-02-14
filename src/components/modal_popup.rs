use dioxus::{core::use_drop, logger::tracing::debug, prelude::*};
use gloo::{
    events::EventListener,
    utils::{format::JsValueSerdeExt, window},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::{
    CardType,
    components::{
        card_details::CardDetailsPopup,
        card_search::{CardSearchPopup, Filters},
    },
    sources::CommonCard,
};

static MODAL_POPUP_ID: GlobalSignal<usize> = Signal::global(|| 1);
static MODAL_POPUP_LIST: GlobalSignal<Vec<(usize, Popup)>> = Signal::global(Vec::new);

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Popup {
    CardDetails(CommonCard, CardType),
    CardSearch(Filters),
}

pub fn show_popup(popup: Popup) {
    let id = *MODAL_POPUP_ID.read();
    *MODAL_POPUP_ID.write() += 1;

    let mut popups = MODAL_POPUP_LIST.write();
    popups.push((id, popup));

    // Push new state to history
    let history = window().history().expect("no history");
    let state = JsValue::from_serde(&*popups).expect("failed to serialize popup list");
    history
        .push_state_with_url(&state, "", None)
        .expect("push history");
}

#[component]
pub fn ModalPopupStack() -> Element {
    // Global popstate listener
    let _listener = use_signal(|| {
        EventListener::new(&window(), "popstate", |_| {
            debug!("Global popstate event");
            let history = window().history().expect("no history");
            if let Ok(state) = history.state() {
                let new_list: Vec<(usize, Popup)> = state.into_serde().unwrap_or_default();
                *MODAL_POPUP_LIST.write() = new_list;
            }
        })
    });

    let popups = MODAL_POPUP_LIST.read();
    let popups_list = popups.iter().map(|(id, popup)| match popup {
        Popup::CardDetails(common_card, card_type) => rsx! {
            CardDetailsPopup {
                key: "{id}",
                popup_id: *id,
                card: common_card.clone(),
                card_type: *card_type,
            }
        },
        Popup::CardSearch(filters) => rsx! {
            CardSearchPopup {
                key: "{id}",
                popup_id: *id,
                default_filters: filters.clone(),
            }
        },
    });

    rsx! {
        {popups_list}
    }
}

pub fn modify_popup_layer(delta: i32) {
    // Track popup layers and prevent scrolling when modals are open
    let window = web_sys::window().expect("window not found");
    let document = window.document().expect("document not found");
    let html = document
        .query_selector("html")
        .expect("failed to query selector")
        .expect("html element not found");

    // Increment or decrement the layer counter based on visibility
    let mut layer = html
        .get_attribute("data-popup-layer")
        .and_then(|a| a.parse::<i32>().ok())
        .unwrap_or(0);

    layer += delta;

    html.set_attribute("data-popup-layer", &layer.to_string())
        .unwrap();

    // Disable scrolling when at least one popup is open
    if layer >= 1 {
        html.set_class_name("is-clipped");
    } else {
        html.set_class_name("");
    }
}

#[component]
pub fn ModelPopup(
    popup_id: usize,
    title: Option<Element>,
    content: Element,
    footer: Option<Element>,
    modal_class: Option<String>,
) -> Element {
    let modal_card = title.is_some() || footer.is_some();
    let modal_class = modal_class.unwrap_or_default();

    // Manage layer counting on mount/unmount
    use_effect(move || {
        modify_popup_layer(1);
    });

    use_drop(move || {
        modify_popup_layer(-1);
    });

    // Close action: navigating back in history
    let close = move |_| {
        let history = window().history().expect("no history");
        // We use back() to close. The popstate listener will update the list.
        history.back().unwrap();
    };

    rsx! {
        div { class: "modal is-active",
            div { class: "modal-background", onclick: close }
            if modal_card {
                div { class: "modal-card", class: "{modal_class}",
                    div { class: "modal-card-details" }
                    if let Some(title) = title {
                        header {
                            class: "modal-card-head p-5",
                            style: "box-shadow: none;",
                            p { class: "modal-card-title is-flex-shrink-1", {title} }
                            button {
                                r#type: "button",
                                "aria-label": "close",
                                class: "delete is-large",
                                onclick: close,
                            }
                        }
                    }
                    section { class: "modal-card-body pt-1", {content} }
                    if let Some(footer) = footer {
                        footer { class: "modal-card-foot", {footer} }
                    }
                }
            } else {
                div { class: "modal-content", {content} }
                button {
                    r#type: "button",
                    "aria-label": "close",
                    class: "modal-close is-large",
                    onclick: close,
                }
            }
        }
    }
}
