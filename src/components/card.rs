use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsInfo;
use num_format::{Locale, ToFormattedString};
use serde::Serialize;
use web_time::{Duration, Instant};

use crate::{
    CardLanguage, CardType,
    sources::{CommonCard, CommonDeck, price_check::PriceCache},
    tracker::{EventType, track_event, track_url},
};

// TODO add popup for card info (allow adding and removing cards)

#[component]
pub fn Card(
    card: CommonCard,
    card_type: CardType,
    card_lang: Signal<CardLanguage>,
    is_preview: bool,
    info: Signal<CardsInfo>,
    common_deck: Option<Signal<CommonDeck>>,
    is_edit: Signal<bool>,
    show_price: Option<Signal<bool>>,
    prices: Option<Signal<PriceCache>>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
    }

    let img_class = if card_type == CardType::Oshi {
        "card-img-oshi"
    } else {
        "card-img"
    };

    let error_img_path: &str = match card_type {
        CardType::Oshi | CardType::Cheer => "cheer-back.webp",
        CardType::Main => "card-back.webp",
    };
    let error_img_path =
        format!("https://qrimpuff.github.io/hocg-fan-sim-assets/img/{error_img_path}");

    let img_path = card
        .image_path(&info.read(), *card_lang.read())
        .unwrap_or_else(|| error_img_path.clone());

    let show_price = show_price.map(|s| *s.read()).unwrap_or(false);
    let price = if let Some(prices) = prices {
        card.price(&info.read(), &prices.read())
            .map(|p| p.to_formatted_string(&Locale::en))
            .unwrap_or("?".into())
    } else {
        "?".into()
    };
    // TODO not only yuyutei
    let price_url = card
        .card_info(&info.read())
        .and_then(|c| c.yuyutei_sell_url.clone());

    // verify card amount
    let total_amount = if let Some(common_deck) = common_deck {
        common_deck
            .read()
            .all_cards()
            .filter(|c| c.card_number == card.card_number)
            .map(|c| c.amount)
            .sum::<u32>()
    } else {
        0
    };
    let max_amount = card.card_info(&info.read()).map(|i| i.max).unwrap_or(50);
    let warning_amount = total_amount > max_amount;
    let warning_class = if warning_amount {
        "is-warning"
    } else {
        "is-dark"
    };

    let tooltip = card
        .card_info(&info.read())
        .map(|e| format!("{} ({})", card.card_number, e.rare))
        .unwrap_or(card.card_number.to_string());

    let _card = card.clone();
    let mut tracking_sent_add_card: Signal<Option<Instant>> = use_signal(|| None);
    let add_card = move |_| {
        if let Some(mut common_deck) = common_deck {
            let mut deck = common_deck.write();
            let mut card = _card.clone();
            card.amount = 1;
            deck.add_card(card, card_type, &info.read());

            if tracking_sent_add_card
                .peek()
                .as_ref()
                .map(|t| t.elapsed() >= Duration::from_secs(10))
                .unwrap_or(true)
            {
                track_event(
                    EventType::EditDeck,
                    EventData {
                        action: "Add card".into(),
                    },
                );
                *tracking_sent_add_card.write() = Some(Instant::now());
            }
        }
    };
    let _card = card.clone();
    let mut tracking_sent_remove_card: Signal<Option<Instant>> = use_signal(|| None);
    let remove_card = move |_| {
        if let Some(mut common_deck) = common_deck {
            let mut deck = common_deck.write();
            let mut card = _card.clone();
            card.amount = 1;
            deck.remove_card(card, card_type, &info.read());

            if tracking_sent_remove_card
                .peek()
                .as_ref()
                .map(|t| t.elapsed() >= Duration::from_secs(10))
                .unwrap_or(true)
            {
                track_event(
                    EventType::EditDeck,
                    EventData {
                        action: "Remove card".into(),
                    },
                );
                *tracking_sent_remove_card.write() = Some(Instant::now());
            }
        }
    };

    rsx! {
        div { class: "m-2",
            figure { class: "image {img_class}",
                img {
                    title: "{tooltip}",
                    border_radius: "3.7%",
                    src: "{img_path}",
                    "onerror": "this.src='{error_img_path}'",
                }
                if show_price {
                    span {
                        class: "badge is-bottom {warning_class}",
                        style: "z-index: 10",
                        " ¥{price} × {card.amount} "
                        if let Some(price_url) = price_url {
                            a {
                                title: "Go to Yuyutei for {card.card_number}",
                                href: "{price_url}",
                                target: "_blank",
                                onclick: |_| { track_url("Yuyutei") },
                                i { class: "fa-solid fa-arrow-up-right-from-square" }
                            }
                        }
                    }
                } else if card_type != CardType::Oshi && card.amount > 0 {
                    span {
                        class: "badge is-bottom {warning_class}",
                        style: "z-index: 10",
                        "{card.amount}"
                    }
                }
            }
            if *is_edit.read() {
                div { class: "mt-1 is-flex is-justify-content-center",
                    if card.card_type(&info.read()) == Some(CardType::Oshi) || card_type == CardType::Oshi {
                        if card.amount > 0 {
                            button {
                                r#type: "button",
                                class: "button is-small has-text-danger",
                                title: "Remove oshi {tooltip}",
                                onclick: remove_card,
                                "Remove"
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "button is-small has-text-success",
                                title: "Select oshi {tooltip}",
                                onclick: add_card,
                                "Select"
                            }
                        }
                    } else {
                        div { class: "buttons has-addons",
                            button {
                                r#type: "button",
                                class: "button is-small",
                                title: "Remove 1 {tooltip}",
                                // disable when no more to remove
                                disabled: card.amount == 0,
                                onclick: remove_card,
                                span { class: "icon is-small has-text-danger",
                                    if card.amount == 1 && is_preview {
                                        // only for deck preview
                                        i { class: "fas fa-trash" }
                                    } else {
                                        i { class: "fas fa-minus" }
                                    }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "button is-small",
                                title: "Add 1 {tooltip}",
                                // disable when reaching max amount. not total amount. allows some buffer for deck building
                                disabled: card.amount >= max_amount,
                                onclick: add_card,
                                span { class: "icon is-small has-text-success",
                                    i { class: "fas fa-plus" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
