use dioxus::{prelude::*, web::WebEventExt};
use gloo::{events::EventListener, utils::window};

fn update_tooltip_position(root: &web_sys::Element, tooltip: &web_sys::Element) {
    let root_rect = root.get_bounding_client_rect();
    let tooltip_rect = tooltip.get_bounding_client_rect();
    let padding = 8.0;
    let desired_left = (root_rect.width() - tooltip_rect.width()) / 2.0;
    let min_left = padding - root_rect.left();
    let max_left = window().inner_width().ok().and_then(|width| width.as_f64()).map_or(
        desired_left,
        |inner_width| inner_width - padding - root_rect.left() - tooltip_rect.width(),
    );
    let tooltip_left = desired_left.clamp(min_left, max_left);
    let arrow_left = (root_rect.width() / 2.0 - tooltip_left).clamp(10.0, tooltip_rect.width() - 10.0);

    tooltip
        .set_attribute(
            "style",
            &format!(
                "left: {tooltip_left}px; transform: none; --tooltip-arrow-left: {arrow_left}px;"
            ),
        )
        .expect("set tooltip style");
}

#[component]
pub fn Tooltip(tooltip: String, children: Element) -> Element {
    let mut root_ref = use_signal(|| None::<web_sys::Element>);
    let mut tooltip_ref = use_signal(|| None::<web_sys::Element>);
    let mut resize_listener = use_signal(|| None::<EventListener>);

    let update_position = {
        let root_ref = root_ref;
        let tooltip_ref = tooltip_ref;
        move || {
            let Some(root) = root_ref.read().as_ref().cloned() else {
                return;
            };
            let Some(tooltip) = tooltip_ref.read().as_ref().cloned() else {
                return;
            };
            update_tooltip_position(&root, &tooltip);
        }
    };

    use_effect(move || {
        let Some(root) = root_ref.read().as_ref().cloned() else {
            return;
        };
        let Some(tooltip) = tooltip_ref.read().as_ref().cloned() else {
            return;
        };

        update_tooltip_position(&root, &tooltip);

        let listener = EventListener::new(&window(), "resize", move |_| {
            update_tooltip_position(&root, &tooltip);
        });
        *resize_listener.write() = Some(listener);
    });

    rsx! {
        span {
            class: "custom-tooltip",
            onmount: move |elem| {
                *root_ref.write() = Some(elem.as_web_event());
            },
            onmouseenter: move |_| update_position(),
            onfocusin: move |_| update_position(),
            ontouchstart: move |_| update_position(),
            {children}
            span {
                class: "custom-tooltip-content",
                onmount: move |elem| {
                    *tooltip_ref.write() = Some(elem.as_web_event());
                },
                "{tooltip}"
            }
        }
    }
}