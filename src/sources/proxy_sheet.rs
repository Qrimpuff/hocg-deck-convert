use std::collections::HashMap;
use std::error::Error;
use std::iter;

use ::image::imageops::FilterType;
use ::image::ImageFormat;
use dioxus::prelude::*;
use printpdf::*;

use super::{CardsInfoMap, CommonDeck};
use crate::download_file;

#[derive(Clone, Copy)]
enum PaperSize {
    A4,
    Letter,
}

async fn generate_pdf(
    deck: &CommonDeck,
    map: &CardsInfoMap,
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
    let margin = Mm(5.0);
    let gap = Mm(0.1);

    let fit_width = (page_width - margin - margin) / (CARD_WIDTH + gap);
    let fit_height = (page_height - margin - margin) / (CARD_HEIGHT + gap);
    // TODO maybe auto rotate
    // let fit_width_side = (page_width - margin - margin) / CARD_HEIGHT;
    // let fit_height_side = (page_height - margin - margin) / CARD_WIDTH;

    let cards_per_page = (fit_width.floor() * fit_height.floor()) as usize;
    let cards: Box<dyn Iterator<Item = &crate::CommonCards>> = if include_cheers {
        Box::new(deck.all_cards())
    } else {
        Box::new(std::iter::once(&deck.oshi).chain(deck.main_deck.iter()))
    };
    let cards: Vec<_> = cards
        .flat_map(|c| iter::repeat(c.clone()).take(c.amount as usize))
        .collect();
    let pages_count = (cards.len() as f32 / cards_per_page as f32).ceil() as usize;

    let title = format!("Proxy sheet for {}", deck.required_deck_name());
    let doc = PdfDocument::empty(&title);

    // download the images (the browser should have them cached)
    let mut img_cache = HashMap::new();
    for card in deck.all_cards() {
        let img_path = {
            if let Some(manage_id) = &card.manage_id {
                if let Some(card) = map.get(&manage_id.parse::<u32>().unwrap()) {
                    card.img.clone()
                } else {
                    // skip missing card
                    continue;
                }
            } else {
                // skip missing card
                continue;
            }
        };

        let url = format!("https://qrimpuff.github.io/hocg-fan-sim-assets/img/{img_path}");
        let image_bytes = reqwest::get(&url).await.unwrap().bytes().await.unwrap();

        let image = ::image::load_from_memory_with_format(&image_bytes, ImageFormat::WebP)?;
        let image = image.resize_exact(CARD_WIDTH_PX, CARD_HEIGHT_PX, FilterType::CatmullRom);
        let image = Image::from_dynamic_image(&image);
        img_cache.insert(card.manage_id.clone(), image);
    }

    for page_idx in 0..pages_count {
        let (page, layer) = doc.add_page(page_width, page_height, "layer");
        let page = doc.get_page(page);
        let current_layer = page.get_layer(layer);

        let mut manage_id = None;
        let mut image_transforms = vec![];

        for card_idx in 0..cards_per_page {
            let Some(card) = cards.get(page_idx * cards_per_page + card_idx) else {
                break;
            };

            // apply transforms
            if manage_id != card.manage_id && !image_transforms.is_empty() {
                if let Some(image) = img_cache.get(&manage_id) {
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
            manage_id = card.manage_id.clone();
            image_transforms.push(ImageTransform {
                dpi: Some(DPI),
                translate_x: Some(
                    margin + (CARD_WIDTH + gap) * (card_idx % fit_width.floor() as usize) as f32,
                ),
                translate_y: Some(
                    page_height
                        - margin
                        - (CARD_HEIGHT + gap)
                            * (1.0 + (card_idx / fit_width.floor() as usize) as f32),
                ),
                ..Default::default()
            });
        }

        // apply transforms
        if !image_transforms.is_empty() {
            if let Some(image) = img_cache.get(&manage_id) {
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
pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
    let mut deck_error = use_signal(String::new);
    let mut paper_size = use_signal(|| PaperSize::A4);
    let mut include_cheers = use_signal(|| false);
    let mut loading = use_signal(|| false);

    let print_deck = move |_| async move {
        let common_deck = common_deck.read();
        let Some(common_deck) = common_deck.as_ref() else {
            return;
        };

        *loading.write() = true;
        *deck_error.write() = String::new();

        let file_name = common_deck.file_name();
        let file_name = format!("{file_name}.proxy_sheet.pdf");
        match generate_pdf(
            common_deck,
            &map.read(),
            *paper_size.read(),
            *include_cheers.read(),
        )
        .await
        {
            Ok(file) => download_file(&file_name, &file[..]),
            Err(e) => *deck_error.write() = e.to_string(),
        }

        *loading.write() = false;
    };

    rsx! {
        div { class: "field",
            label { "for": "paper_size", class: "label", "Paper size" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "paper_size",
                        oninput: move |ev| {
                            *paper_size
                                .write() = match ev.value().as_str() {
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
                            *include_cheers
                                .write() = match ev.value().as_str() {
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
                    disabled: common_deck.read().is_none() || *loading.read(),
                    onclick: print_deck,
                    span { class: "icon",
                        i { class: "fa-solid fa-print" }
                    }
                    span { "Print deck to PDF" }
                }
            }
        }
    }
}
