use dioxus::{prelude::*, web::WebEventExt};
use gloo::{events::EventListener, utils::window};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TooltipPlacement {
    Top,
    Right,
}

fn update_tooltip_position(
    root: &web_sys::Element,
    tooltip: &web_sys::Element,
    placement: TooltipPlacement,
) {
    let root_rect = root.get_bounding_client_rect();
    let tooltip_rect = tooltip.get_bounding_client_rect();
    let padding = 8.0;
    let window_width = window()
        .inner_width()
        .ok()
        .and_then(|width| width.as_f64())
        .unwrap_or(root_rect.right() + tooltip_rect.width() + padding);
    let window_height = window()
        .inner_height()
        .ok()
        .and_then(|height| height.as_f64())
        .unwrap_or(root_rect.bottom() + tooltip_rect.height() + padding);

    let style = match placement {
        TooltipPlacement::Top => {
            let desired_left = (root_rect.width() - tooltip_rect.width()) / 2.0;
            let min_left = padding - root_rect.left();
            let max_left = window_width - padding - root_rect.left() - tooltip_rect.width();
            let tooltip_left = desired_left.clamp(min_left, max_left);
            let arrow_left =
                (root_rect.width() / 2.0 - tooltip_left).clamp(10.0, tooltip_rect.width() - 10.0);

            format!(
                "left: {tooltip_left}px; top: auto; bottom: calc(100% + 0.45rem); transform: none; --tooltip-arrow-left: {arrow_left}px;"
            )
        }
        TooltipPlacement::Right => {
            let gap = 8.0;
            let desired_left = root_rect.width() + gap;
            let min_left = padding - root_rect.left();
            let max_left = window_width - padding - root_rect.left() - tooltip_rect.width();
            let tooltip_left = desired_left.clamp(min_left, max_left);

            let desired_top = (root_rect.height() - tooltip_rect.height()) / 2.0;
            let min_top = padding - root_rect.top();
            let max_top = window_height - padding - root_rect.top() - tooltip_rect.height();
            let tooltip_top = desired_top.clamp(min_top, max_top);
            let arrow_top =
                (root_rect.height() / 2.0 - tooltip_top).clamp(10.0, tooltip_rect.height() - 10.0);

            format!(
                "left: {tooltip_left}px; top: {tooltip_top}px; bottom: auto; transform: none; --tooltip-arrow-top: {arrow_top}px;"
            )
        }
    };

    tooltip.set_attribute("style", &style).expect("set tooltip style");
}

#[component]
pub fn Tooltip(
    tooltip: String,
    children: Element,
    #[props(default = true)] underline: bool,
    #[props(default = TooltipPlacement::Top)] placement: TooltipPlacement,
) -> Element {
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
            update_tooltip_position(&root, &tooltip, placement);
        }
    };

    use_effect(move || {
        let Some(root) = root_ref.read().as_ref().cloned() else {
            return;
        };
        let Some(tooltip) = tooltip_ref.read().as_ref().cloned() else {
            return;
        };

        update_tooltip_position(&root, &tooltip, placement);

        let listener = EventListener::new(&window(), "resize", move |_| {
            update_tooltip_position(&root, &tooltip, placement);
        });
        *resize_listener.write() = Some(listener);
    });

    rsx! {
        span {
            class: "custom-tooltip",
            class: if !underline { "custom-tooltip-no-underline" },
            class: if placement == TooltipPlacement::Right { "custom-tooltip-right" },
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