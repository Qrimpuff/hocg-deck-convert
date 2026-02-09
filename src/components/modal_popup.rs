use dioxus::{core::use_drop, logger::tracing::debug, prelude::*};
use gloo::{events::EventListener, utils::window};
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

pub enum Popup {
    CardDetails(CommonCard, CardType),
    CardSearch(Filters),
}

pub fn show_popup(popup: Popup) {
    let id = *MODAL_POPUP_ID.read();
    *MODAL_POPUP_ID.write() += 1;
    MODAL_POPUP_LIST.write().push((id, popup));
}

#[component]
pub fn ModalPopupStack() -> Element {
    let popups = MODAL_POPUP_LIST.read();
    let popups = popups.iter().map(|(id, popup)| match popup {
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
        {popups}
    }
}

pub fn update_popup_layer(show_popup: Option<Signal<bool>>) -> Option<EventListener> {
    debug!("update_popup_layer {:?}", show_popup);

    // Track popup layers and prevent scrolling when modals are open
    let html = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("html")
        .unwrap()
        .unwrap();

    // Increment or decrement the layer counter based on visibility
    let mut layer = html
        .get_attribute("data-popup-layer")
        .and_then(|a| a.parse().ok())
        .unwrap_or(0);
    let visible = show_popup.is_some_and(|s| *s.read());
    layer += if visible { 1 } else { -1 };
    html.set_attribute("data-popup-layer", &layer.to_string())
        .unwrap();

    // Disable scrolling when at least one popup is open
    if layer == 1 {
        html.set_class_name("is-clipped");
    } else if layer == 0 {
        html.set_class_name("");
    }

    // History management for back button behavior
    let history = window().history().expect("no history");
    if visible {
        if layer == 1 {
            history
                .push_state_with_url(&JsValue::TRUE, "", None)
                .expect("push history");
        }

        // Create event listener to close popup on browser back button
        let href = window().location().href().unwrap();
        Some(EventListener::new(&window(), "popstate", move |_| {
            // Store current state before potential changes
            let prev_layer = layer;
            let prev_href = href.clone();
            debug!("from popstate");

            // Get updated state after popstate event
            let href = window().location().href().unwrap();
            let html = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("html")
                .unwrap()
                .unwrap();
            let layer = html
                .get_attribute("data-popup-layer")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);

            // Check if we need to close this popup based on unchanged state
            if prev_layer == layer && prev_href == href {
                // For nested popups, maintain history state
                if layer > 1 {
                    history
                        .push_state_with_url(&JsValue::TRUE, "", None)
                        .expect("push history");
                }
                // Close the current popup
                *show_popup.expect("signal should be there").write() = false;
            }
        }))
    } else {
        // Clean up history state when closing the last popup
        if layer == 0 && history.state().unwrap() == JsValue::TRUE {
            history.back().unwrap();
        }
        None
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
    let mut show_popup = use_signal(|| true);
    let mut popup_listener: Signal<Option<EventListener>> = use_signal(|| None);

    // Remove popup from global list when closed
    let _ = use_effect(move || {
        if !*show_popup.read() {
            let mut popups = MODAL_POPUP_LIST.write();
            if let Some(pos) = popups.iter().position(|(id, _)| *id == popup_id) {
                popups.remove(pos);
            }
        }
    });

    let _ = use_effect(move || {
        // Update the popup layer and manage history state (back button behavior)
        // only update if the state changes
        if *show_popup.read() && popup_listener.peek().is_none()
            || !*show_popup.read() && popup_listener.peek().is_some()
        {
            *popup_listener.write() = update_popup_layer(Some(show_popup));
        }
    });

    // Clean up the popup layer on unmount
    use_drop(move || {
        if popup_listener.peek().is_some() {
            let _ = update_popup_layer(None);
        }
    });

    rsx! {
        if *show_popup.read() {
            div { class: "modal is-active",
                div {
                    class: "modal-background",
                    onclick: move |_| { show_popup.set(false) },
                }
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
                                    onclick: move |_| { show_popup.set(false) },
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
                        onclick: move |_| { show_popup.set(false) },
                    }
                }
            }
        }
    }
}
