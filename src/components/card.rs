use std::{cell::RefCell, collections::HashMap};

use base64::Engine;
use dioxus::prelude::*;
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
use serde::Serialize;

use crate::{
    AUTO_SAVE_DECK, CARDS_PRICES, CardLanguage, CardType, EXPORT_FORMAT, FREE_BASIC_CHEERS,
    PREVIEW_CARD_LANG, PRICE_SERVICE,
    components::modal_popup::{Popup, show_popup},
    sources::{
        CommonCard, DeckLike, DeckOrPile, DeckType, ImageOptions, csv,
        price_check::PriceCheckService,
    },
    tracker::{EventType, TrackEvent, track_event, track_external_url},
};

/// Full list of cards the popup can step through, provided by the list owner.
pub type CardNavList = Signal<Vec<(CommonCard, CardType)>>;

/// Size the placeholder is decoded at. The preview is smooth, so the browser can stretch a
/// small image to the card size without it looking different.
const PLACEHOLDER_WIDTH: usize = 20;
const PLACEHOLDER_HEIGHT: usize = 28;

/// Draws RGBA pixels (row-major from the top) as a BMP data URI, dropping the alpha.
fn bmp_data_uri(rgba: &[u8]) -> String {
    const WIDTH: usize = PLACEHOLDER_WIDTH;
    const HEIGHT: usize = PLACEHOLDER_HEIGHT;
    const HEADER: usize = 54;
    // BMP rows are padded to a multiple of 4 bytes
    const ROW: usize = (WIDTH * 3).div_ceil(4) * 4;

    let size = HEADER + ROW * HEIGHT;
    let mut bmp = Vec::with_capacity(size);
    // file header
    bmp.extend(b"BM");
    bmp.extend((size as u32).to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    bmp.extend((HEADER as u32).to_le_bytes());
    // BITMAPINFOHEADER
    bmp.extend(40u32.to_le_bytes());
    bmp.extend((WIDTH as i32).to_le_bytes());
    bmp.extend((HEIGHT as i32).to_le_bytes());
    bmp.extend(1u16.to_le_bytes());
    bmp.extend(24u16.to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    bmp.extend(((ROW * HEIGHT) as u32).to_le_bytes());
    bmp.extend(2835i32.to_le_bytes());
    bmp.extend(2835i32.to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    // pixels: bottom-up rows, BGR
    for row in rgba.as_chunks::<{ WIDTH * 4 }>().0.iter().rev() {
        for px in row.as_chunks::<4>().0 {
            bmp.extend([px[2], px[1], px[0]]);
        }
        bmp.resize(bmp.len() + ROW - WIDTH * 3, 0);
    }

    format!(
        "data:image/bmp;base64,{}",
        base64::prelude::BASE64_STANDARD.encode(bmp)
    )
}

/// Blurred image of the card shown while it loads, from the illustration's CardHash.
/// `None` when the illustration has none (empty in older databases) or it can't be decoded.
/// Decoded once per illustration: decoding takes tens of microseconds, and cards build their
/// image on every render.
fn placeholder(img_placeholder: &str) -> Option<String> {
    thread_local! {
        static CACHE: RefCell<HashMap<String, Option<String>>> = RefCell::default();
    }
    if img_placeholder.is_empty() {
        return None;
    }
    CACHE.with_borrow_mut(|cache| {
        cache
            .entry(img_placeholder.to_owned())
            .or_insert_with(|| {
                hocg::cardhash::decode(
                    img_placeholder,
                    PLACEHOLDER_WIDTH as u32,
                    PLACEHOLDER_HEIGHT as u32,
                )
                .map(|rgba| bmp_data_uri(&rgba))
            })
            .clone()
    })
}

/// Transparent image with the size of a card, shown instead of the card back so the
/// placeholder stays visible. It keeps the card ratio once loaded, unlike a 1x1 image.
/// Quotes are escaped so it can be used inside the `onerror` script.
const TRANSPARENT_IMG: &str = "data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%22400%22 height=%22560%22/%3E";

/// Attributes of a card `img`: the image, what to show if it fails, and the placeholder
/// drawn behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct CardImageSource {
    pub src: String,
    pub onerror: String,
    pub background_image: String,
}

impl CardImageSource {
    /// When the illustration has a placeholder, it is shown instead of the card back if
    /// there is no image or if it fails to load.
    pub fn new(
        card: Option<&CommonCard>,
        img_path: Option<String>,
        card_type: CardType,
        db: &CardsDatabase,
    ) -> Self {
        let card_back = match card.and_then(|c| c.card_type(db)).unwrap_or(card_type) {
            CardType::Oshi | CardType::Cheer => "/hocg-deck-convert/assets/cheer-back.webp",
            CardType::Main => "/hocg-deck-convert/assets/card-back.webp",
        };

        let placeholder = card
            .and_then(|c| c.card_illustration(db))
            .and_then(|illust| placeholder(&illust.img_placeholder));

        match placeholder {
            Some(placeholder) => Self {
                src: img_path.unwrap_or_else(|| TRANSPARENT_IMG.into()),
                onerror: format!("this.onerror=null;this.src='{TRANSPARENT_IMG}'"),
                background_image: format!("url(\"{placeholder}\")"),
            },
            None => Self {
                src: img_path.unwrap_or_else(|| card_back.into()),
                onerror: format!("this.onerror=null;this.src='{card_back}'"),
                background_image: "none".into(),
            },
        }
    }
}

/// Card image drawn over its placeholder. Keyed on the source, so a new image replaces the
/// previous one right away (showing the placeholder) instead of when it loads.
///
/// Loaded lazily unless `eager`, so lists hidden with `display: none` (e.g. the card search)
/// don't load their images with the page. In Firefox, a page controlled by the service worker
/// has to load and decode every image again on reload, so each image loaded at startup delays
/// the visible ones. Images in a scroll container load only once scrolled into view, so
/// `eager` is for images that are always shown when mounted.
#[component]
pub fn CardImg(
    img: CardImageSource,
    #[props(into)] style: Option<String>,
    #[props(default)] eager: bool,
) -> Element {
    let loading = if eager { "eager" } else { "lazy" };
    // A single style attribute: mixing it with style properties (e.g. `background_size`)
    // lets it overwrite them
    let style = format!(
        "border-radius: 4.9% / 3.5%; background-image: {}; background-size: 100% 100%; background-repeat: no-repeat; {}",
        img.background_image,
        style.unwrap_or_default()
    );
    rsx! {
        for img in std::iter::once(img) {
            img {
                key: "{img.src}",
                style: "{style}",
                width: "400",
                height: "560",
                loading: loading,
                src: "{img.src}",
                "onerror": "{img.onerror}",
            }
        }
    }
}

#[component]
pub fn Card(
    card: CommonCard,
    card_type: CardType,
    card_lang: Signal<CardLanguage>,
    is_preview: bool,
    image_options: ImageOptions,
    db: Signal<CardsDatabase>,
    common_deck: Option<Signal<DeckOrPile>>,
    is_edit: Signal<bool>,
    is_details_edit: Option<Signal<bool>>,
    show_price: Option<Signal<bool>>,
    card_details: Option<Signal<CommonCard>>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        action: String,
    }
    impl TrackEvent for EventData {}

    let is_details_edit = is_details_edit.unwrap_or(is_edit);

    let deck_is_pile = common_deck
        .as_ref()
        .is_some_and(|d| matches!(*d.read(), DeckOrPile::Pile(_)));

    let img_class = if card_type == CardType::Oshi {
        "card-img-oshi"
    } else {
        "card-img"
    };

    let img = CardImageSource::new(
        Some(&card),
        card.image_path(&db.read(), *card_lang.read(), image_options),
        card_type,
        &db.read(),
    );

    let price_service = *PRICE_SERVICE.read();
    let show_price = show_price.map(|s| *s.read()).unwrap_or(false);
    let free_basic_cheers = *FREE_BASIC_CHEERS.read();
    let (price_key, price) = card
        .price_display(
            &db.read(),
            &CARDS_PRICES.read(),
            price_service,
            free_basic_cheers,
        )
        .unwrap_or((None, "?".into()));
    let price_url = price_key.map(|key| key.service_id().url().clone());
    let price_name = price_service.name();

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
    let max_amount = if !deck_is_pile
        || matches!(
            *EXPORT_FORMAT.read(),
            Some(DeckType::DeckLog | DeckType::HoloDelta | DeckType::TabletopSim)
        ) {
        card.max_amount(*card_lang.read(), &db.read())
    } else {
        u32::MAX
    };
    let warning_amount = total_amount > max_amount;
    let warning_amount_class = if warning_amount {
        "is-warning"
    } else {
        "is-dark"
    };

    // highlight cards that cause the warnings
    let is_unknown = card.is_unknown(&db.read());
    let is_unreleased = card.is_unreleased(*PREVIEW_CARD_LANG.read(), &db.read());
    let is_warning_card = match *EXPORT_FORMAT.read() {
        Some(DeckType::DeckLog) => is_unknown || is_unreleased,
        Some(DeckType::HoloDelta) => is_unknown,
        Some(DeckType::TabletopSim) => is_unknown || is_unreleased,
        Some(DeckType::ProxySheets) => {
            card.card_type(&db.read()) != Some(CardType::Cheer)
                && card
                    .image_path(
                        &db.read(),
                        *PREVIEW_CARD_LANG.read(),
                        if is_preview {
                            ImageOptions::proxy_validation()
                        } else {
                            ImageOptions::proxy_validation_strict()
                        },
                    )
                    .is_none()
        }
        Some(DeckType::Csv) => !csv::notes(&card, *PREVIEW_CARD_LANG.read(), &db.read()).is_empty(),
        Some(DeckType::PriceCheck) => {
            card.card_illustration(&db.read())
                .is_none_or(|i| match *PRICE_SERVICE.read() {
                    PriceCheckService::Yuyutei => i.yuyutei_sell_paths.is_none(),
                    PriceCheckService::TcgPlayer => i.tcgplayer_product_ids.is_none(),
                })
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
            deck.add_card(card, card_type, &db.read(), !is_preview);

            track_event(
                EventType::EditDeck,
                EventData {
                    action: "Add card".into(),
                },
            );

            AUTO_SAVE_DECK.write().replace(deck.clone());
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

            AUTO_SAVE_DECK.write().replace(deck.clone());
        }
    };

    let nav_cards = try_consume_context::<CardNavList>();

    let _card = card.clone();
    let is_selected = use_memo(move || {
        if let Some(card_details) = card_details {
            *card_details.read() == _card
        } else {
            false
        }
    });

    rsx! {
        div { class: "m-1 my-2",
            figure {
                class: "image {img_class}",
                class: if *is_selected.read() { "selected" },
                class: if is_warning_card && !*is_selected.read() { "warning" },
                a {
                    href: "#",
                    role: "button",
                    title: "Show card details for {tooltip}",
                    onclick: move |evt| {
                        evt.prevent_default();
                        if let Some(mut card_details) = card_details {
                            card_details.set(card.clone());
                            track_event(
                                EventType::EditDeck,
                                EventData {
                                    action: "Card illustration".into(),
                                },
                            );
                        } else {
                            show_popup(Popup::CardDetails {
                                card: card.clone(),
                                card_type,
                                is_edit: *is_details_edit.read(),
                                show_price,
                                nav_cards: nav_cards
                                    .map(|nav| nav.read().to_vec())
                                    .unwrap_or_default(),
                            });
                            track_event(
                                EventType::EditDeck,
                                EventData {
                                    action: "Card details".into(),
                                },
                            );
                        }
                    },
                    CardImg { img }
                }
                if show_price {
                    span {
                        class: "badge is-bottom {warning_amount_class} card-amount",
                        style: "z-index: 10",
                        " {price} × {card.amount} "
                        if let Some(price_url) = price_url && is_preview {
                            a {
                                title: "Go to {price_name} for {card.card_number}",
                                href: "{price_url}",
                                target: "_blank",
                                rel: "noopener",
                                onclick: |_| { track_external_url(price_name) },
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
                    if !deck_is_pile
                        && (card.card_type(&db.read()) == Some(CardType::Oshi)
                            || card_type == CardType::Oshi)
                    {
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
                                    >= if deck_is_pile {
                                        100
                                    } else {
                                        match card.card_info(&db.read()).map(|info| info.card_type) {
                                            Some(hocg::CardType::OshiHoloMember) => 1,
                                            Some(hocg::CardType::Cheer) => 20,
                                            _ => 50,
                                        }
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

#[cfg(test)]
mod tests {
    use super::{bmp_data_uri, placeholder};
    use base64::{Engine, prelude::BASE64_STANDARD};
    use hocg_fan_sim_assets_model::cardhash;

    #[test]
    fn bmp_from_rgba_pixels() {
        let mut rgba = vec![0u8; 20 * 28 * 4];
        // top-left pixel is red, bottom-left pixel is blue
        rgba[0] = 255;
        rgba[27 * 20 * 4 + 2] = 255;

        let uri = bmp_data_uri(&rgba);
        let bmp = BASE64_STANDARD
            .decode(uri.strip_prefix("data:image/bmp;base64,").unwrap())
            .unwrap();
        // 20 BGR pixels are 60 bytes, already a multiple of 4
        let size = 54 + 60 * 28;
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(bmp.len(), size);
        assert_eq!(
            u32::from_le_bytes(bmp[2..6].try_into().unwrap()),
            size as u32
        );
        // first stored row is the bottom one, in BGR
        assert_eq!(&bmp[54..57], &[255, 0, 0]);
        // last stored row is the top one
        assert_eq!(&bmp[54 + 60 * 27..54 + 60 * 27 + 3], &[0, 0, 255]);
    }

    #[test]
    fn placeholder_from_card_hash() {
        let hash = cardhash::encode(&[120u8; 40 * 56 * 3], 40, 56);
        let image = placeholder(&hash).unwrap();
        assert!(image.starts_with("data:image/bmp;base64,"));

        // older databases have no placeholder
        assert_eq!(placeholder(""), None);
        assert_eq!(placeholder("v3|H32=abc|CYM=FF8000"), None);
    }
}
