use dioxus::{logger::tracing::debug, prelude::*};
use gloo::{events::EventListener, utils::window};
use wasm_bindgen::JsValue;

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
pub fn ModelPopup(show_popup: Signal<bool>, content: Element) -> Element {
    let mut popup_listener = use_signal(|| None);

    let _ = use_effect(move || {
        // Update the popup layer and manage history state (back button behavior)
        // only update if the state changes
        if *show_popup.read() && popup_listener.peek().is_none()
            || !*show_popup.read() && popup_listener.peek().is_some()
        {
            *popup_listener.write() = update_popup_layer(Some(show_popup));
        }
    });

    use_drop(move || {
        if *show_popup.peek() {
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
