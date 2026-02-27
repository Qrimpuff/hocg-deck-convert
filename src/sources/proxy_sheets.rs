use std::error::Error;
use std::io::Cursor;
use std::{collections::HashMap, sync::Arc};

use ::image::imageops::FilterType;
use ::image::ImageFormat;
use dioxus::prelude::*;
use futures::future::try_join_all;
use futures::lock::Mutex;
use printpdf::*;
use serde::Serialize;

use super::{CardsDatabase, CommonDeck, ImageOptions};
use crate::components::deck_validation::{has_missing_proxies, DeckValidation};
use crate::{download_file, track_event, CardLanguage, EventType, PREVIEW_CARD_LANG};

#[derive(Clone, Copy, Serialize)]
enum PaperSize {
    A4,
    Letter,
}

#[derive(Clone, Copy)]
struct Layout {
    page_width: Mm,
    page_height: Mm,
    margin_x: Mm,
    margin_y: Mm,
    gap: Mm,
    fit_w: usize,
    fit_h: usize,
    cards_per_page: usize,
}

impl Layout {
    fn compute(page_width: Mm, page_height: Mm, card_w: Mm, card_h: Mm) -> Self {
        let mut margin_x = Mm(5.0);
        let mut margin_y = Mm(5.0);
        let gap = Mm(0.5);

        let fit_w_f = ((page_width - margin_x - margin_x) / (card_w + gap)).floor();
        let fit_h_f = ((page_height - margin_y - margin_y) / (card_h + gap)).floor();

        let fit_w = fit_w_f.max(0.0) as usize;
        let fit_h = fit_h_f.max(0.0) as usize;

        if fit_w == 0 || fit_h == 0 {
            return Self {
                page_width,
                page_height,
                margin_x,
                margin_y,
                gap,
                fit_w,
                fit_h,
                cards_per_page: 0,
            };
        }

        margin_x = (page_width - (card_w + gap) * (fit_w as f32)) / 2.0;
        margin_y = (page_height - (card_h + gap) * (fit_h as f32)) / 2.0;

        Self {
            page_width,
            page_height,
            margin_x,
            margin_y,
            gap,
            fit_w,
            fit_h,
            cards_per_page: fit_w * fit_h,
        }
    }

    fn card_translate(&self, idx_in_page: usize, card_w: Mm, card_h: Mm) -> (Mm, Mm) {
        let col = idx_in_page % self.fit_w;
        let row = idx_in_page / self.fit_w;

        let x = self.margin_x + (card_w + self.gap) * (col as f32);

        // PDF origin is bottom-left; place rows from top to bottom.
        let y = self.page_height
            - self.margin_y
            - (card_h + self.gap) * (1.0 + row as f32);

        (x, y)
    }
}

async fn generate_pdf(
    deck: &CommonDeck,
    db: &CardsDatabase,
    card_lang: CardLanguage,
    paper_size: PaperSize,
    include_cheers: bool,
    include_cropmarks: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // ==== Output / card constants ====
    const DPI: f32 = 300.0;
    const INCH_PER_MM: f32 = 0.0393701;
    const CARD_WIDTH: Mm = Mm(63.5);
    const CARD_HEIGHT: Mm = Mm(88.9);

    let card_width_px: u32 = (DPI * INCH_PER_MM * CARD_WIDTH.0) as u32;
    let card_height_px: u32 = (DPI * INCH_PER_MM * CARD_HEIGHT.0) as u32;

    let (page_width, page_height) = match paper_size {
        PaperSize::A4 => (Mm(210.0), Mm(297.0)),
        PaperSize::Letter => (Mm(215.9), Mm(279.4)),
    };

    let layout = Layout::compute(page_width, page_height, CARD_WIDTH, CARD_HEIGHT);
    if layout.cards_per_page == 0 {
        return Err("Paper size is too small to fit any card with current margins/gap".into());
    }

    // ==== Build the list of cards to print (expanded by amount) ====
    let base_iter: Box<dyn Iterator<Item = &crate::CommonCard>> = if include_cheers {
        Box::new(deck.all_cards())
    } else {
        Box::new(deck.oshi.iter().chain(deck.main_deck.iter()))
    };

    let cards: Vec<crate::CommonCard> = base_iter
        .filter(|c| c.image_path(db, card_lang, ImageOptions::proxy_print()).is_some())
        .flat_map(|c| std::iter::repeat_n(c.clone(), c.amount as usize))
        .collect();

    let pages_count = (cards.len() as f32 / layout.cards_per_page as f32).ceil() as usize;

    // ==== Create PDF document ====
    let title = format!("Proxy sheets for {}", deck.required_deck_name(db));
    let mut doc = PdfDocument::new(&title);
    doc.metadata.info.producer = "hololive OCG Deck Converter".to_string();

    // ==== Download and cache images once per unique card (deck.all_cards()) ====
    // Keyed by (card_number, illustration_idx). illustration_idx is Option<usize> in your model.
    type CacheKey<'a> = Option<(&'a String, Option<usize>)>;

    let img_cache: Arc<Mutex<HashMap<CacheKey<'_>, RawImage>>> =
        Arc::new(Mutex::new(HashMap::with_capacity(cards.len())));

    let download_tasks = deck.all_cards().map(|card| {
        let img_cache = img_cache.clone();
        async move {
            let Some(img_path) = card.image_path(db, card_lang, ImageOptions::proxy_print()) else {
                return Ok::<(), Box<dyn Error>>(());
            };

            let image_bytes = reqwest::get(&img_path).await?.bytes().await?;
            let image = ::image::load_from_memory_with_format(&image_bytes, ImageFormat::WebP)?;
            let image = image.resize_exact(card_width_px, card_height_px, FilterType::CatmullRom);

            // Convert to PNG bytes, then decode into printpdf RawImage.
            let mut bytes = Cursor::new(vec![]);
            ::image::write_buffer_with_format(
                &mut bytes,
                image.as_bytes(),
                card_width_px,
                card_height_px,
                image.color(),
                ImageFormat::Png,
            )?;

            let raw =
                RawImage::decode_from_bytes_async(&bytes.into_inner(), &mut vec![]).await?;

            let key: CacheKey<'_> = Some((&card.card_number, card.illustration_idx));
            img_cache.lock().await.insert(key, raw);

            Ok(())
        }
    });
    try_join_all(download_tasks).await?;

    let img_cache = Arc::try_unwrap(img_cache).unwrap().into_inner();

    // printpdf `add_image` returns an `XObjectId` which is what `Op::UseXobject` expects.
    let image_ids: HashMap<CacheKey<'_>, XObjectId> = img_cache
        .into_iter()
        .map(|(key, image)| (key, doc.add_image(&image)))
        .collect();

    // ==== Optional cropmarks overlay (raster PNG) ====
    let overlay_id: Option<XObjectId> = if include_cropmarks {
        fn mm_to_px(dpi: f32, mm: f32) -> i32 {
            (dpi * 0.0393701 * mm).round() as i32
        }

        fn draw_line_thick(
            img: &mut ::image::RgbaImage,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            thickness: i32,
            color: ::image::Rgba<u8>,
        ) {
            // Bresenham with a square brush for thickness.
            let w = img.width() as i32;
            let h = img.height() as i32;

            let mut x = x1;
            let mut y = y1;
            let dx = (x2 - x1).abs();
            let dy = -(y2 - y1).abs();
            let sx = if x1 < x2 { 1 } else { -1 };
            let sy = if y1 < y2 { 1 } else { -1 };
            let mut err = dx + dy;

            let r = (thickness / 2).max(0);

            loop {
                for oy in -r..=r {
                    for ox in -r..=r {
                        let px = x + ox;
                        let py = y + oy;
                        if px >= 0 && px < w && py >= 0 && py < h {
                            img.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }

                if x == x2 && y == y2 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y += sy;
                }
            }
        }

        fn corner_l(
            dpi: f32,
            img: &mut ::image::RgbaImage,
            x_mm: f32,
            y_mm: f32,
            dir_x: i32,
            dir_y: i32,
            l_mm: f32,
            gap_mm: f32,
            stroke_mm: f32,
            color: ::image::Rgba<u8>,
        ) {
            let l = mm_to_px(dpi, l_mm);
            let gap = mm_to_px(dpi, gap_mm);
            let stroke = mm_to_px(dpi, stroke_mm).max(1);

            let x = mm_to_px(dpi, x_mm);
            let y = mm_to_px(dpi, y_mm);

            let hx1 = if dir_x < 0 { x - gap - l } else { x + gap };
            let hx2 = if dir_x < 0 { x - gap } else { x + gap + l };
            draw_line_thick(img, hx1, y, hx2, y, stroke, color);

            let vy1 = if dir_y < 0 { y - gap - l } else { y + gap };
            let vy2 = if dir_y < 0 { y - gap } else { y + gap + l };
            draw_line_thick(img, x, vy1, x, vy2, stroke, color);
        }

        fn corner_cross(
            dpi: f32,
            img: &mut ::image::RgbaImage,
            x_mm: f32,
            y_mm: f32,
            dir_x: i32,
            dir_y: i32,
            len_mm: f32,
            gap_mm: f32,
            stroke_mm: f32,
            color: ::image::Rgba<u8>,
        ) {
            corner_l(
                dpi, img, x_mm, y_mm, dir_x, dir_y, len_mm, gap_mm, stroke_mm, color,
            );
            corner_l(
                dpi, img, x_mm, y_mm, -dir_x, -dir_y, len_mm, gap_mm, stroke_mm, color,
            );
        }

        let page_w_px = mm_to_px(DPI, page_width.0).max(1) as u32;
        let page_h_px = mm_to_px(DPI, page_height.0).max(1) as u32;
        let mut overlay =
            ::image::RgbaImage::from_pixel(page_w_px, page_h_px, ::image::Rgba([0, 0, 0, 0]));

        // Cropmark parameters (in mm)
        let l_mm = 3.0_f32;
        let stroke_mm = 0.25_f32;
        let inner_l_mm = 1.5_f32;
        let gap_mm = 0.0_f32;
        let bleed_mm = 0.0_f32;
        let color = ::image::Rgba([0x68, 0x68, 0x68, 0xFF]);

        // NOTE: This overlay is currently drawn for a 3x3 grid only.
        if layout.fit_w >= 3 && layout.fit_h >= 3 {
            for row in 0..3 {
                for col in 0..3 {
                    let slot_x =
                        layout.margin_x.0 + (CARD_WIDTH.0 + layout.gap.0) * (col as f32);
                    let slot_y =
                        layout.margin_y.0 + (CARD_HEIGHT.0 + layout.gap.0) * (row as f32);

                    let x_left_cut = slot_x + bleed_mm;
                    let x_right_cut = x_left_cut + CARD_WIDTH.0;
                    let y_top_cut = slot_y + bleed_mm;
                    let y_bot_cut = y_top_cut + CARD_HEIGHT.0;

                    if row == 0 && col == 0 {
                        corner_l(
                            DPI,
                            &mut overlay,
                            x_left_cut,
                            y_top_cut,
                            -1,
                            -1,
                            l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        continue;
                    }
                    if row == 0 && col == 2 {
                        corner_l(
                            DPI,
                            &mut overlay,
                            x_right_cut,
                            y_top_cut,
                            1,
                            -1,
                            l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        continue;
                    }
                    if row == 2 && col == 0 {
                        corner_l(
                            DPI,
                            &mut overlay,
                            x_left_cut,
                            y_bot_cut,
                            -1,
                            1,
                            l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        continue;
                    }
                    if row == 2 && col == 2 {
                        corner_l(
                            DPI,
                            &mut overlay,
                            x_right_cut,
                            y_bot_cut,
                            1,
                            1,
                            l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        continue;
                    }

                    if row == 0 && col == 1 {
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_left_cut),
                            mm_to_px(DPI, y_top_cut - l_mm),
                            mm_to_px(DPI, x_left_cut),
                            mm_to_px(DPI, y_top_cut),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_right_cut),
                            mm_to_px(DPI, y_top_cut - l_mm),
                            mm_to_px(DPI, x_right_cut),
                            mm_to_px(DPI, y_top_cut),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        continue;
                    }

                    if row == 2 && col == 1 {
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_left_cut),
                            mm_to_px(DPI, y_bot_cut),
                            mm_to_px(DPI, x_left_cut),
                            mm_to_px(DPI, y_bot_cut + l_mm),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_right_cut),
                            mm_to_px(DPI, y_bot_cut),
                            mm_to_px(DPI, x_right_cut),
                            mm_to_px(DPI, y_bot_cut + l_mm),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        continue;
                    }

                    if row == 1 && col == 0 {
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_left_cut - l_mm),
                            mm_to_px(DPI, y_top_cut),
                            mm_to_px(DPI, x_left_cut),
                            mm_to_px(DPI, y_top_cut),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_left_cut - l_mm),
                            mm_to_px(DPI, y_bot_cut),
                            mm_to_px(DPI, x_left_cut),
                            mm_to_px(DPI, y_bot_cut),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        continue;
                    }

                    if row == 1 && col == 2 {
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_right_cut),
                            mm_to_px(DPI, y_top_cut),
                            mm_to_px(DPI, x_right_cut + l_mm),
                            mm_to_px(DPI, y_top_cut),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        draw_line_thick(
                            &mut overlay,
                            mm_to_px(DPI, x_right_cut),
                            mm_to_px(DPI, y_bot_cut),
                            mm_to_px(DPI, x_right_cut + l_mm),
                            mm_to_px(DPI, y_bot_cut),
                            mm_to_px(DPI, stroke_mm).max(1),
                            color,
                        );
                        continue;
                    }

                    if row == 1 && col == 1 {
                        corner_cross(
                            DPI,
                            &mut overlay,
                            x_right_cut,
                            y_top_cut,
                            1,
                            -1,
                            inner_l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        corner_cross(
                            DPI,
                            &mut overlay,
                            x_right_cut,
                            y_bot_cut,
                            1,
                            1,
                            inner_l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        corner_cross(
                            DPI,
                            &mut overlay,
                            x_left_cut,
                            y_bot_cut,
                            -1,
                            1,
                            inner_l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        corner_cross(
                            DPI,
                            &mut overlay,
                            x_left_cut,
                            y_top_cut,
                            -1,
                            -1,
                            inner_l_mm,
                            gap_mm,
                            stroke_mm,
                            color,
                        );
                        continue;
                    }
                }
            }
        }

        let mut overlay_bytes = Cursor::new(vec![]);
        ::image::DynamicImage::ImageRgba8(overlay).write_to(&mut overlay_bytes, ImageFormat::Png)?;
        let overlay_raw =
            RawImage::decode_from_bytes_async(&overlay_bytes.into_inner(), &mut vec![]).await?;
        Some(doc.add_image(&overlay_raw))
    } else {
        None
    };

    // ==== Build pages ====
    let pages = (0..pages_count)
        .map(|page_idx| {
            let mut ops = Vec::new();

            for idx_in_page in 0..layout.cards_per_page {
                let global_idx = page_idx * layout.cards_per_page + idx_in_page;
                let Some(card) = cards.get(global_idx) else { break };

                let key: CacheKey<'_> = Some((&card.card_number, card.illustration_idx));
                if let Some(image_id) = image_ids.get(&key) {
                    let (tx, ty) = layout.card_translate(idx_in_page, CARD_WIDTH, CARD_HEIGHT);
                    ops.push(Op::UseXobject {
                        id: image_id.clone(),
                        transform: XObjectTransform {
                            dpi: Some(DPI),
                            translate_x: Some(tx.into()),
                            translate_y: Some(ty.into()),
                            ..Default::default()
                        },
                    });
                }
            }

            // Do not move overlay_id into the closure; clone the inner id for each page.
            if let Some(oid) = overlay_id.clone() {
                ops.push(Op::UseXobject {
                    id: oid,
                    transform: XObjectTransform {
                        dpi: Some(DPI),
                        translate_x: Some(Mm(0.0).into()),
                        translate_y: Some(Mm(0.0).into()),
                        ..Default::default()
                    },
                });
            }

            PdfPage::new(page_width, page_height, ops)
        })
        .collect::<Vec<_>>();

    Ok(doc.with_pages(pages).save(
        &PdfSaveOptions {
            image_optimization: Some(ImageOptimizationOptions {
                max_image_size: None,
                ..Default::default()
            }),
            ..Default::default()
        },
        &mut Vec::new(),
    ))
}

#[component]
pub fn Export(mut common_deck: Signal<CommonDeck>, db: Signal<CardsDatabase>) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        language: CardLanguage,
        missing_proxies: bool,
        paper_size: PaperSize,
        include_cheers: bool,
        include_cropmarks: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let mut deck_error = use_signal(String::new);
    let card_lang = PREVIEW_CARD_LANG.signal();
    let mut paper_size = use_signal(|| PaperSize::A4);
    let mut include_cheers = use_signal(|| false);
    let mut include_cropmarks = use_signal(|| true);
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
        let cm = if *include_cropmarks.read() { "cm" } else { "nocm" };

        let file_name = common_deck.file_name(&db.read());
        let file_name = format!("{file_name}.proxy_sheets.{lang}_{ps}.{cm}.pdf");

        let missing_proxies = has_missing_proxies(&common_deck, &db.read(), *card_lang.read());

        match generate_pdf(
            &common_deck,
            &db.read(),
            *card_lang.read(),
            *paper_size.read(),
            *include_cheers.read(),
            *include_cropmarks.read(),
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
                        missing_proxies,
                        paper_size: *paper_size.read(),
                        include_cheers: *include_cheers.read(),
                        include_cropmarks: *include_cropmarks.read(),
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
                        missing_proxies,
                        paper_size: *paper_size.read(),
                        include_cheers: *include_cheers.read(),
                        include_cropmarks: *include_cropmarks.read(),
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
            label { "for": "include_cropmarks", class: "label", "Cropmarks" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "include_cropmarks",
                        oninput: move |ev| {
                            *include_cropmarks.write() = match ev.value().as_str() {
                                "no" => false,
                                "yes" => true,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "yes", "Yes" }
                        option { value: "no", "No" }
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