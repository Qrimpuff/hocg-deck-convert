use dioxus::prelude::*;
use hocg_fan_sim_assets_model::CardsDatabase;
use num_format::{Locale, ToFormattedString};
use serde::Serialize;

use crate::{
    CARD_DETAILS, CardLanguage, CardType, EXPORT_FORMAT, SHOW_CARD_DETAILS,
    sources::{CommonCard, CommonDeck, DeckType, price_check::PriceCache},
    tracker::{EventType, track_event, track_url},
};

#[component]
pub fn Card(
    card: CommonCard,
    card_type: CardType,
    card_lang: Signal<CardLanguage>,
    is_preview: bool,
    db: Signal<CardsDatabase>,
    common_deck: Option<Signal<CommonDeck>>,
    is_edit: Signal<bool>,
    show_price: Option<Signal<bool>>,
    prices: Option<Signal<PriceCache>>,
    card_detail: Option<Signal<CommonCard>>,
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
    let error_img_path = format!("/hocg-deck-convert/assets/{error_img_path}");

    let img_path = card
        .image_path(&db.read(), *card_lang.read(), true, true)
        .unwrap_or_else(|| error_img_path.clone());

    let show_price = show_price.map(|s| *s.read()).unwrap_or(false);
    let price = if let Some(prices) = prices {
        card.price(&db.read(), &prices.read())
            .map(|p| p.to_formatted_string(&Locale::en))
            .unwrap_or("?".into())
    } else {
        "?".into()
    };
    // TODO not only yuyutei
    let price_url = card
        .card_illustration(&db.read())
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
    let max_amount = card
        .card_info(&db.read())
        .map(|i| i.max_amount)
        .unwrap_or(50);
    let warning_amount = total_amount > max_amount;
    let warning_amount_class = if warning_amount {
        "is-warning"
    } else {
        "is-dark"
    };

    // highlight cards that cause the warnings
    let is_unknown = card.is_unknown(&db.read());
    let is_unreleased = card.is_unreleased(&db.read());
    let is_warning_card = if is_preview {
        let export = *EXPORT_FORMAT.read();
        if is_unknown {
            matches!(
                export,
                Some(DeckType::DeckLog)
                    | Some(DeckType::HoloDelta)
                    | Some(DeckType::HoloDuel)
                    | Some(DeckType::TabletopSim)
            )
        } else if is_unreleased {
            matches!(
                export,
                Some(DeckType::DeckLog) | Some(DeckType::HoloDuel) | Some(DeckType::TabletopSim)
            )
        } else {
            false
        }
    } else {
        false
    };

    let tooltip = card
        .card_illustration(&db.read())
        .map(|e| format!("{} ({})", card.card_number, e.rarity))
        .unwrap_or(card.card_number.to_string());

    let _card = card.clone();
    let add_card = move |_| {
        if let Some(mut common_deck) = common_deck {
            let mut deck = common_deck.write();
            let mut card = _card.clone();
            card.amount = 1;
            deck.add_card(card, card_type, &db.read());

            track_event(
                EventType::EditDeck,
                EventData {
                    action: "Add card".into(),
                },
            );
        }
    };
    let _card = card.clone();
    let remove_card = move |_| {
        if let Some(mut common_deck) = common_deck {
            let mut deck = common_deck.write();
            let mut card = _card.clone();
            card.amount = 1;
            deck.remove_card(card, card_type, &db.read());

            track_event(
                EventType::EditDeck,
                EventData {
                    action: "Remove card".into(),
                },
            );
        }
    };

    let _card = card.clone();
    let is_selected = use_memo(move || {
        if let Some(card_detail) = card_detail {
            *card_detail.read() == _card
        } else {
            false
        }
    });

    rsx! {
        div { class: "m-2",
            figure {
                class: "image {img_class}",
                class: if *is_selected.read() { "selected" },
                class: if is_warning_card { "warning" },
                a {
                    href: "#",
                    role: "button",
                    title: "Show card details for {tooltip}",
                    onclick: move |evt| {
                        evt.prevent_default();
                        if let Some(mut card_detail) = card_detail {
                            card_detail.set(card.clone());
                            track_event(
                                EventType::EditDeck,
                                EventData {
                                    action: "Card illustration".into(),
                                },
                            );
                        } else {
                            *CARD_DETAILS.write() = Some((card.clone(), card_type));
                            *SHOW_CARD_DETAILS.write() = true;
                            track_event(
                                EventType::EditDeck,
                                EventData {
                                    action: "Card details".into(),
                                },
                            );
                        }
                    },
                    img {
                        border_radius: "3.7%",
                        src: "{img_path}",
                        "onerror": "this.src='{error_img_path}'",
                    }
                }
                if show_price {
                    span {
                        class: "badge is-bottom {warning_amount_class} card-amount",
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
                        class: "badge is-bottom {warning_amount_class} card-amount",
                        style: "z-index: 10",
                        "{card.amount}"
                    }
                }
            }
        }
        if *is_edit.read() {
            div { class: "mt-1 is-flex is-justify-content-center",
                if card.card_type(&db.read()) == Some(CardType::Oshi) || card_type == CardType::Oshi {
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
