use std::error::Error;
use std::{collections::HashMap, sync::Arc};

use ::image::ImageFormat;
use ::image::imageops::FilterType;
use dioxus::prelude::*;
use futures::future::try_join_all;
use futures::lock::Mutex;
use printpdf::*;
use serde::Serialize;

use super::{CardsDatabase, CommonDeck};
use crate::components::deck_validation::DeckValidation;
use crate::sources::ImageOptions;
use crate::{CardLanguage, EventType, PREVIEW_CARD_LANG, download_file, track_event};

#[derive(Clone, Copy, Serialize)]
enum PaperSize {
    A4,
    Letter,
}

async fn generate_pdf(
    deck: &CommonDeck,
    db: &CardsDatabase,
    card_lang: CardLanguage,
    paper_size: PaperSize,
    include_cheers: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    const DPI: f32 = 300.0;
    const CARD_WIDTH: Mm = Mm(63.5);
    const CARD_HEIGHT: Mm = Mm(88.9);
    // dot per millimeter
    const CARD_WIDTH_PX: u32 = (DPI * 0.0393701 * CARD_WIDTH.0) as u32;
    const CARD_HEIGHT_PX: u32 = (DPI * 0.0393701 * CARD_HEIGHT.0) as u32;

    let (page_width, page_height) = match paper_size {
        PaperSize::A4 => (Mm(210.0), Mm(297.0)),
        PaperSize::Letter => (Mm(215.9), Mm(279.4)),
    };
    // TODO maybe custom margin and gap
    let mut margin_width = Mm(5.0);
    let mut margin_height = Mm(5.0);
    let gap = Mm(0.1);

    let fit_width = ((page_width - margin_width - margin_width) / (CARD_WIDTH + gap)).floor();
    let fit_height = ((page_height - margin_height - margin_height) / (CARD_HEIGHT + gap)).floor();
    // TODO maybe auto rotate
    // let fit_width_side = (page_width - margin - margin) / CARD_HEIGHT;
    // let fit_height_side = (page_height - margin - margin) / CARD_WIDTH;

    // center the cards
    margin_width = (page_width - (CARD_WIDTH + gap) * fit_width) / 2.0;
    margin_height = (page_height - (CARD_HEIGHT + gap) * fit_height) / 2.0;

    let cards_per_page = (fit_width * fit_height) as usize;
    let cards: Box<dyn Iterator<Item = &crate::CommonCard>> = if include_cheers {
        Box::new(deck.all_cards())
    } else {
        Box::new(deck.oshi.iter().chain(deck.main_deck.iter()))
    };
    let cards: Vec<_> = cards
        .filter(|c| {
            c.image_path(db, card_lang, ImageOptions::proxy_print())
                .is_some()
        })
        .flat_map(|c| std::iter::repeat_n(c.clone(), c.amount as usize))
        .collect();
    let pages_count = (cards.len() as f32 / cards_per_page as f32).ceil() as usize;

    let title = format!("Proxy sheets for {}", deck.required_deck_name(db));
    let doc = PdfDocument::empty(&title);

    // download the images (the browser should have them cached)
    let img_cache = Arc::new(Mutex::new(HashMap::with_capacity(cards.len())));
    let download_images = deck.all_cards().map(|card| {
        let img_cache = img_cache.clone();
        async move {
            let Some(img_path) = card.image_path(db, card_lang, ImageOptions::proxy_print()) else {
                // skip missing card
                return Ok::<(), Box<dyn Error>>(());
            };

            let image_bytes = reqwest::get(&img_path).await?.bytes().await?;

            let image = ::image::load_from_memory_with_format(&image_bytes, ImageFormat::WebP)?;
            let image = image.resize_exact(CARD_WIDTH_PX, CARD_HEIGHT_PX, FilterType::CatmullRom);
            let image = Image::from_dynamic_image(&image);
            let cache_key = Some((&card.card_number, card.illustration_idx));
            img_cache.lock().await.insert(cache_key, image);

            Ok(())
        }
    });
    try_join_all(download_images).await?;

    let img_cache = img_cache.lock().await;
    for page_idx in 0..pages_count {
        let (page, layer) = doc.add_page(page_width, page_height, "layer");
        let page = doc.get_page(page);
        let current_layer = page.get_layer(layer);

        let mut cache_key = None;
        let mut image_transforms = vec![];

        for card_idx in 0..cards_per_page {
            let Some(card) = cards.get(page_idx * cards_per_page + card_idx) else {
                break;
            };

            // apply transforms
            let card_cache_key = Some((&card.card_number, card.illustration_idx));
            if cache_key != card_cache_key && !image_transforms.is_empty() {
                if let Some(image) = img_cache.get(&cache_key) {
                    let image = Image {
                        image: image.image.clone(),
                        smask: image.smask.clone(),
                    };
                    image.add_to_layer_with_many_transforms(
                        current_layer.clone(),
                        &image_transforms,
                    );
                }
                image_transforms.clear();
            }

            // place the image on the page
            cache_key = card_cache_key;
            image_transforms.push(ImageTransform {
                dpi: Some(DPI),
                translate_x: Some(
                    margin_width + (CARD_WIDTH + gap) * (card_idx % fit_width as usize) as f32,
                ),
                translate_y: Some(
                    page_height
                        - margin_height
                        - (CARD_HEIGHT + gap) * (1.0 + (card_idx / fit_width as usize) as f32),
                ),
                ..Default::default()
            });
        }

        // apply transforms
        if !image_transforms.is_empty() {
            if let Some(image) = img_cache.get(&cache_key) {
                let image = Image {
                    image: image.image.clone(),
                    smask: image.smask.clone(),
                };
                image.add_to_layer_with_many_transforms(current_layer.clone(), &image_transforms);
            }
        }
    }

    Ok(doc.save_to_bytes()?)
}

#[component]
pub fn Export(mut common_deck: Signal<CommonDeck>, db: Signal<CardsDatabase>) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        language: CardLanguage,
        paper_size: PaperSize,
        include_cheers: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let mut deck_error = use_signal(String::new);
    let card_lang = PREVIEW_CARD_LANG.signal();
    let mut paper_size = use_signal(|| PaperSize::A4);
    let mut include_cheers = use_signal(|| false);
    let mut loading = use_signal(|| false);

    let print_deck = move |_| async move {
        let common_deck = common_deck.read();

        *loading.write() = true;
        *deck_error.write() = String::new();

        let lang = match *card_lang.read() {
            CardLanguage::Japanese => "jp",
            CardLanguage::English => "en",
        };
        let ps = match *paper_size.read() {
            PaperSize::A4 => "a4",
            PaperSize::Letter => "letter",
        };
        let file_name = common_deck.file_name(&db.read());
        let file_name = format!("{file_name}.proxy_sheets.{lang}_{ps}.pdf");
        match generate_pdf(
            &common_deck,
            &db.read(),
            *card_lang.read(),
            *paper_size.read(),
            *include_cheers.read(),
        )
        .await
        {
            Ok(file) => {
                download_file(&file_name, &file[..]);
                track_event(
                    EventType::Export("Proxy sheets".into()),
                    EventData {
                        format: "Proxy sheets",
                        language: *card_lang.read(),
                        paper_size: *paper_size.read(),
                        include_cheers: *include_cheers.read(),
                        error: None,
                    },
                );
            }
            Err(e) => {
                *deck_error.write() = e.to_string();
                track_event(
                    EventType::Export("Proxy sheets".into()),
                    EventData {
                        format: "Proxy sheets",
                        language: *card_lang.read(),
                        paper_size: *paper_size.read(),
                        include_cheers: *include_cheers.read(),
                        error: Some(e.to_string()),
                    },
                );
            }
        }

        *loading.write() = false;
    };

    rsx! {
        DeckValidation {
            deck_check: true,
            proxy_check: true,
            allow_unreleased: true,
            card_lang,
            db,
            common_deck,
        }

        div { class: "field",
            label { "for": "card_language", class: "label", "Card language" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "card_language",
                        oninput: move |ev| {
                            *PREVIEW_CARD_LANG.write() = match ev.value().as_str() {
                                "jp" => CardLanguage::Japanese,
                                "en" => CardLanguage::English,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "en", "English" }
                        option { value: "jp", "Japanese" }
                    }
                }
            }
        }

        div { class: "field",
            label { "for": "paper_size", class: "label", "Paper size" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "paper_size",
                        oninput: move |ev| {
                            *paper_size.write() = match ev.value().as_str() {
                                "a4" => PaperSize::A4,
                                "letter" => PaperSize::Letter,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "a4", "A4 (21.0x29.7 cm)" }
                        option { value: "letter", "Letter (8.5x11.0 in)" }
                    }
                }
            }
        }

        div { class: "field",
            label { "for": "include_cheers", class: "label", "Include cheers" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "include_cheers",
                        oninput: move |ev| {
                            *include_cheers.write() = match ev.value().as_str() {
                                "no" => false,
                                "yes" => true,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "no", "No" }
                        option { value: "yes", "Yes" }
                    }
                }
            }
        }

        div { class: "field",
            div { class: "control",
                button {
                    r#type: "button",
                    class: "button",
                    class: if *loading.read() { "is-loading" },
                    disabled: common_deck.read().is_empty() || *loading.read(),
                    onclick: print_deck,
                    span { class: "icon",
                        i { class: "fa-solid fa-print" }
                    }
                    span { "Print deck to PDF" }
                }
            }
            p { class: "help is-danger", "{deck_error}" }
        }
    }
}
