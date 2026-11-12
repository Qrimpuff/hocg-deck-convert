use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use dioxus::prelude::*;
use hocg_fan_sim_assets_model::{self as hocg, CardsDatabase};
use serde::Serialize;

use crate::{
    AUTO_SAVE_DECK, CARDS_PRICES, CardLanguage, CardType, EXPORT_FORMAT, FREE_BASIC_CHEERS,
    PREVIEW_CARD_LANG, PRICE_SERVICE,
    components::modal_popup::{Popup, show_popup},
    sources::{
        CommonCard, DeckLike, DeckOrPile, DeckType, ImageOptions, price_check::PriceCheckService,
    },
    tracker::{EventType, TrackEvent, track_event, track_external_url},
};

/// Full list of cards the popup can step through, provided by the list owner.
pub type CardNavList = Signal<Vec<(CommonCard, CardType)>>;

const GRID_WIDTH: usize = 10;
const GRID_HEIGHT: usize = 14;

/// Decodes the v3 `G` component: a 10x14 grid of average RGB colors, row-major from the top.
fn color_grid(img_hash: &str) -> Option<Vec<u8>> {
    let grid = img_hash
        .split('|')
        .find_map(|part| part.strip_prefix("G="))?;
    let grid = BASE64_STANDARD_NO_PAD.decode(grid).ok()?;
    (grid.len() == GRID_WIDTH * GRID_HEIGHT * 3).then_some(grid)
}

/// Draws the color grid as a BMP data URI. The browser smooths it when scaling it up to the
/// card size.
fn grid_placeholder(grid: &[u8]) -> String {
    const WIDTH: usize = GRID_WIDTH;
    const HEIGHT: usize = GRID_HEIGHT;
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
    for row in grid.as_chunks::<{ WIDTH * 3 }>().0.iter().rev() {
        for rgb in row.as_chunks::<3>().0 {
            bmp.extend([rgb[2], rgb[1], rgb[0]]);
        }
        bmp.resize(bmp.len() + ROW - WIDTH * 3, 0);
    }

    format!(
        "data:image/bmp;base64,{}",
        base64::prelude::BASE64_STANDARD.encode(bmp)
    )
}

/// Blurred image of the card shown while it loads, from the color grid of the image hash.
fn placeholder(img_hash: &str) -> Option<String> {
    color_grid(img_hash).map(|grid| grid_placeholder(&grid))
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
            .and_then(|illust| placeholder(&illust.img_hash));

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
#[component]
pub fn CardImg(img: CardImageSource, #[props(into)] style: Option<String>) -> Element {
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
            Some(
                DeckType::DeckLog
                    | DeckType::HoloDelta
                    | DeckType::HoloDuel
                    | DeckType::TabletopSim
            )
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
        Some(DeckType::HoloDuel) => is_unknown || is_unreleased,
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
    use super::{color_grid, grid_placeholder, placeholder};
    use base64::{
        Engine,
        prelude::{BASE64_STANDARD, BASE64_STANDARD_NO_PAD},
    };

    #[test]
    fn placeholder_bitmap_from_grid_component() {
        let mut grid = vec![0u8; 10 * 14 * 3];
        // top-left cell is red, bottom-left cell is blue
        grid[0] = 255;
        grid[13 * 10 * 3 + 2] = 255;
        let g = BASE64_STANDARD_NO_PAD.encode(&grid);

        let decoded = color_grid(&format!("v3|H32=abc|CYM=FF8000|G={g}|L=JP")).unwrap();
        assert_eq!(decoded, grid);
        let uri = grid_placeholder(&decoded);
        let bmp = BASE64_STANDARD
            .decode(uri.strip_prefix("data:image/bmp;base64,").unwrap())
            .unwrap();
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(bmp.len(), 54 + 32 * 14);
        assert_eq!(
            u32::from_le_bytes(bmp[2..6].try_into().unwrap()),
            54 + 32 * 14
        );
        // first stored row is the bottom one, in BGR
        assert_eq!(&bmp[54..57], &[255, 0, 0]);
        // last stored row is the top one
        assert_eq!(&bmp[54 + 32 * 13..54 + 32 * 13 + 3], &[0, 0, 255]);

        let short = BASE64_STANDARD_NO_PAD.encode([0u8; 30]);
        assert_eq!(color_grid(&format!("v3|G={short}")), None);
        assert_eq!(color_grid("v2|H32=abc|CYM=FF8000"), None);
    }

    #[test]
    fn placeholder_needs_grid() {
        let g = BASE64_STANDARD_NO_PAD.encode([10u8; 10 * 14 * 3]);
        let image = placeholder(&format!("v3|H32=abc|CYM=FF8000|G={g}|L=JP")).unwrap();
        assert!(image.starts_with("data:image/bmp;base64,"));

        assert_eq!(placeholder("v2|H32=abc|CYM=FF8000"), None);
    }
}
