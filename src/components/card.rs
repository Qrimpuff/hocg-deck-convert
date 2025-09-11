use dioxus::prelude::*;
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
use serde::Serialize;

use crate::{
    CARD_DETAILS, CARDS_PRICES, CardLanguage, CardType, EXPORT_FORMAT, FREE_BASIC_CHEERS,
    PREVIEW_CARD_LANG, PRICE_SERVICE, SHOW_CARD_DETAILS,
    sources::{CommonCard, CommonDeck, DeckType, ImageOptions, price_check::PriceCheckService},
    tracker::{EventType, track_event, track_url},
};

#[component]
pub fn Card(
    card: CommonCard,
    card_type: CardType,
    card_lang: Signal<CardLanguage>,
    is_preview: bool,
    image_options: ImageOptions,
    db: Signal<CardsDatabase>,
    common_deck: Option<Signal<CommonDeck>>,
    is_edit: Signal<bool>,
    show_price: Option<Signal<bool>>,
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
        .image_path(&db.read(), *card_lang.read(), image_options)
        .unwrap_or_else(|| error_img_path.clone());

    let price_service = *PRICE_SERVICE.read();
    let show_price = show_price.map(|s| *s.read()).unwrap_or(false);
    let free_basic_cheers = *FREE_BASIC_CHEERS.read();
    let price = card
        .price_display(
            &db.read(),
            &CARDS_PRICES.read(),
            price_service,
            free_basic_cheers,
        )
        .unwrap_or("?".into());
    let price_url = card.price_url(&db.read(), price_service);
    let price_name = match price_service {
        PriceCheckService::Yuyutei => "Yuyutei",
        PriceCheckService::TcgPlayer => "TCGplayer",
    };

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
    let max_amount = card.max_amount(*card_lang.read(), &db.read());
    let warning_amount = total_amount > max_amount;
    let warning_amount_class = if warning_amount {
        "is-warning"
    } else {
        "is-dark"
    };

    // highlight cards that cause the warnings
    let is_unknown = card.is_unknown(&db.read());
    let is_unreleased = card.is_unreleased(*PREVIEW_CARD_LANG.read(), &db.read());
    let is_warning_card = match is_preview.then(|| *EXPORT_FORMAT.read()).flatten() {
        Some(DeckType::DeckLog) => is_unknown || is_unreleased,
        Some(DeckType::HoloDelta) => is_unknown,
        Some(DeckType::HoloDuel) => is_unknown || is_unreleased,
        Some(DeckType::TabletopSim) => is_unknown || is_unreleased,
        Some(DeckType::ProxySheets) => {
            card.card_type(&db.read()) != Some(CardType::Cheer)
                && card
                    .image_path(
                        &db.read(),
                        *card_lang.read(),
                        ImageOptions::proxy_validation(),
                    )
                    .is_none()
        }
        _ => false,
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
        div { class: "m-1 my-2",
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
                        " {price} × {card.amount} "
                        if let Some(price_url) = price_url {
                            a {
                                title: "Go to {price_name} for {card.card_number}",
                                href: "{price_url}",
                                target: "_blank",
                                onclick: |_| { track_url(price_name) },
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
                                disabled: card.amount
                                    >= match card.card_info(&db.read()).map(|info| info.card_type) {
                                        Some(hocg::CardType::OshiHoloMember) => 1,
                                        Some(hocg::CardType::Cheer) => 20,
                                        _ => 50,
                                    },
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
