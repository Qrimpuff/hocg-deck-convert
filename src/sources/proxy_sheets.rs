use std::error::Error;
use std::io::Cursor;
use std::{collections::HashMap, sync::Arc};

use ::image::ImageFormat;
use ::image::imageops::FilterType;
use bitflags::bitflags;
use dioxus::prelude::*;
use futures::future::try_join_all;
use futures::lock::Mutex;
use imageproc::drawing::draw_line_segment_mut;
use printpdf::*;
use serde::{Serialize, Serializer};

use super::{CardsDatabase, ImageOptions};
use crate::components::deck_validation::{DeckValidation, has_missing_proxies};
use crate::sources::{DeckLike, DeckOrPile};
use crate::tracker::TrackEvent;
use crate::{
    CardLanguage, EventType, PREVIEW_CARD_LANG, download_file, get_local_country, track_event,
};

const DPI: f32 = 300.0;
const INCH_PER_MM: f32 = 0.0393701;
const DEFAULT_MARGIN: Mm = Mm(4.5);
const DEFAULT_CROP_MARK_THICKNESS: Mm = Mm(0.25);

const DEFAULT_INCLUDE_CHEERS: bool = false;
const DEFAULT_CROP_MARK_SIZE: CropMarksSize = CropMarksSize::Mm(3.0);
const DEFAULT_CROP_MARK_POSITION: CropMarksPosition = CropMarksPosition::Centered;
const DEFAULT_CARD_SIZE: CardSize = CardSize::Metric;
const DEFAULT_GAP: Mm = Mm(0.5);

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
enum PaperSize {
    A4,
    Letter,
    Legal,
}

impl PaperSize {
    fn dimensions(&self) -> (Mm, Mm) {
        match self {
            PaperSize::A4 => (Mm(210.0), Mm(297.0)),
            PaperSize::Letter => (Mm(215.9), Mm(279.4)),
            PaperSize::Legal => (Mm(215.9), Mm(355.6)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
enum CardSize {
    Metric,
    Imperial,
}

impl CardSize {
    fn dimensions(&self) -> (Mm, Mm) {
        match self {
            CardSize::Metric => (Mm(63.0), Mm(88.0)),
            CardSize::Imperial => (Mm(63.5), Mm(88.9)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CropMarksSize {
    None,
    Mm(f32),
    FullLength,
}

impl Serialize for CropMarksSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            CropMarksSize::None => serializer.serialize_str("None"),
            CropMarksSize::Mm(size_mm) => serializer.serialize_str(&format!("{size_mm}mm")),
            CropMarksSize::FullLength => serializer.serialize_str("Full length"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
enum CropMarksPosition {
    Centered,
    CardCorners,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
struct ProxySheetSettings {
    card_lang: CardLanguage,
    paper_size: PaperSize,
    include_cheers: bool,
    crop_marks_size: CropMarksSize,
    crop_marks_position: CropMarksPosition,
    card_size: CardSize,
    gap: Mm,
}

#[derive(Clone, Copy)]
struct Layout {
    page_width: Mm,
    page_height: Mm,
    dpi: f32,
    margin_x: Mm,
    margin_y: Mm,
    gap: Mm,
    rotated: bool,
    card_w: Mm,
    card_h: Mm,
    fit_w: usize,
    fit_h: usize,
    cards_per_page: usize,
    crop_marks_position: CropMarksPosition,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct CropMarkFlags: u8 {
        const NORTH = 1;
        const SOUTH = 1 << 1;
        const WEST = 1 << 2;
        const EAST = 1 << 3;

        const NS = Self::NORTH.bits() | Self::SOUTH.bits();
        const WE = Self::WEST.bits() | Self::EAST.bits();
        const CROSS = Self::NS.bits() | Self::WE.bits();
    }
}

impl Layout {
    fn compute(
        page_width: Mm,
        page_height: Mm,
        dpi: f32,
        card_w: Mm,
        card_h: Mm,
        gap: Mm,
        crop_marks_position: CropMarksPosition,
    ) -> Self {
        let mut margin_x = DEFAULT_MARGIN;
        let mut margin_y = DEFAULT_MARGIN;

        // Compute how many cards fit on the page vertically
        let fit_w = ((page_width - margin_x - margin_x) / (card_w + gap))
            .floor()
            .max(0.0) as usize;
        let fit_h = ((page_height - margin_y - margin_y) / (card_h + gap))
            .floor()
            .max(0.0) as usize;

        // Compute how many cards fit if we rotate the cards
        let rot_fit_w = ((page_width - margin_x - margin_x) / (card_h + gap))
            .floor()
            .max(0.0) as usize;
        let rot_fit_h = ((page_height - margin_y - margin_y) / (card_w + gap))
            .floor()
            .max(0.0) as usize;

        // Choose the orientation that fits more cards on the page
        let (rotated, card_w, card_h, fit_w, fit_h) = if fit_w * fit_h >= rot_fit_w * rot_fit_h {
            (false, card_w, card_h, fit_w, fit_h)
        } else {
            (true, card_h, card_w, rot_fit_w, rot_fit_h)
        };

        // Center the grid on the page
        if fit_w > 0 && fit_h > 0 {
            margin_x = (page_width - (card_w * (fit_w as f32) + gap * ((fit_w - 1) as f32))) / 2.0;
            margin_y = (page_height - (card_h * (fit_h as f32) + gap * ((fit_h - 1) as f32))) / 2.0;
        }

        Self {
            page_width,
            page_height,
            dpi,
            margin_x,
            margin_y,
            gap,
            rotated,
            card_w,
            card_h,
            fit_w,
            fit_h,
            cards_per_page: fit_w * fit_h,
            crop_marks_position,
        }
    }

    /// Returns the bottom-left translation (Mm) for the card slot.
    fn card_translate(&self, idx_in_page: usize) -> (Mm, Mm) {
        let col = idx_in_page % self.fit_w;
        let row = idx_in_page / self.fit_w;

        let x = self.margin_x + (self.card_w + self.gap) * (col as f32);

        // PDF origin is bottom-left; place rows from top to bottom.
        let y = self.page_height
            - self.margin_y
            - (self.card_h + self.gap) * (row as f32)
            - self.card_h;

        (x, y)
    }

    /// Returns the positions of crop marks (Mm) for all card slots.
    fn crop_marks_positions(&self) -> Vec<(Mm, Mm, CropMarkFlags)> {
        let half_gap = self.gap / 2.0;

        let mut positions = Vec::new();

        // If the gap is too small, force crop marks to be centered to avoid overly thick crop marks
        if self.crop_marks_position == CropMarksPosition::Centered
            || self.gap <= DEFAULT_CROP_MARK_THICKNESS
        {
            // Centered crop marks
            for row in 0..self.fit_h {
                for col in 0..self.fit_w {
                    // Cross shapes
                    let flags = if self.crop_marks_position == CropMarksPosition::CardCorners {
                        CropMarkFlags::CROSS
                    } else {
                        (if row == 0 {
                            CropMarkFlags::NORTH
                        } else {
                            CropMarkFlags::empty()
                        } | if col == 0 {
                            CropMarkFlags::WEST
                        } else {
                            CropMarkFlags::empty()
                        })
                    };

                    let idx_in_page = row * self.fit_w + col;
                    let (mut tx, mut ty) = self.card_translate(idx_in_page);

                    // Convert to origin top-left for easier use in the overlay
                    ty = self.page_height - ty - self.card_h;

                    // Adjust for gaps
                    tx -= half_gap;
                    ty -= half_gap;

                    positions.push((
                        tx,
                        ty,
                        // Cross in the centers
                        if flags.is_empty() {
                            CropMarkFlags::CROSS
                        } else {
                            flags
                        },
                    ));

                    // Add bottom and right edges
                    if row == self.fit_h - 1 {
                        let ty = ty + self.card_h + self.gap;
                        let flags = flags | CropMarkFlags::SOUTH;
                        positions.push((tx, ty, flags));
                    }
                    if col == self.fit_w - 1 {
                        let tx = tx + self.card_w + self.gap;
                        let flags = flags | CropMarkFlags::EAST;
                        positions.push((tx, ty, flags));
                    }
                    if row == self.fit_h - 1 && col == self.fit_w - 1 {
                        let ty = ty + self.card_h + self.gap;
                        let tx = tx + self.card_w + self.gap;
                        let flags = flags | CropMarkFlags::SOUTH | CropMarkFlags::EAST;
                        positions.push((tx, ty, flags));
                    }
                }
            }
        } else if self.crop_marks_position == CropMarksPosition::CardCorners {
            // Card corners crop marks
            for idx in 0..self.cards_per_page {
                let thickness = DEFAULT_CROP_MARK_THICKNESS;
                let flags = CropMarkFlags::CROSS;
                let (tx, mut ty) = self.card_translate(idx);

                // Convert to origin top-left for easier use in the overlay
                ty = self.page_height - ty - self.card_h;

                // Top-left corner
                positions.push((tx - thickness / 2.0, ty - thickness / 2.0, flags));
                // Top-right corner
                positions.push((
                    tx + self.card_w + thickness / 2.0,
                    ty - thickness / 2.0,
                    flags,
                ));
                // Bottom-left corner
                positions.push((
                    tx - thickness / 2.0,
                    ty + self.card_h + thickness / 2.0,
                    flags,
                ));
                // Bottom-right corner
                positions.push((
                    tx + self.card_w + thickness / 2.0,
                    ty + self.card_h + thickness / 2.0,
                    flags,
                ));
            }
        } else {
            unreachable!()
        }

        // from top-left top to bottom-right
        positions.sort_by_key(|(tx, ty, _)| (*ty, *tx));
        positions
    }
}

impl CropMarkFlags {
    fn draw_crop_mark(
        &self,
        img: &mut ::image::RgbaImage,
        x: u32,
        y: u32,
        len: u32,
        thickness: u32,
        color: ::image::Rgba<u8>,
    ) {
        let len_x = if self.contains(CropMarkFlags::WE) {
            len / 2
        } else {
            len
        };

        let len_y = if self.contains(CropMarkFlags::NS) {
            len / 2
        } else {
            len
        };

        if self.contains(CropMarkFlags::NORTH) {
            draw_line_thick_mut(img, (x, y), (x, y.saturating_sub(len_y)), thickness, color);
        }
        if self.contains(CropMarkFlags::SOUTH) {
            draw_line_thick_mut(img, (x, y), (x, y.saturating_add(len_y)), thickness, color);
        }
        if self.contains(CropMarkFlags::WEST) {
            draw_line_thick_mut(img, (x, y), (x.saturating_sub(len_x), y), thickness, color);
        }
        if self.contains(CropMarkFlags::EAST) {
            draw_line_thick_mut(img, (x, y), (x.saturating_add(len_x), y), thickness, color);
        }
    }
}

fn draw_line_thick_mut(
    img: &mut ::image::RgbaImage,
    (x1, y1): (u32, u32),
    (x2, y2): (u32, u32),
    thickness: u32,
    color: ::image::Rgba<u8>,
) {
    let r = (thickness / 2) as i32;

    for oy in 0..thickness as i32 {
        for ox in 0..thickness as i32 {
            let x1 = x1 as i32 + ox - r;
            let y1 = y1 as i32 + oy - r;
            let x2 = x2 as i32 + ox - r;
            let y2 = y2 as i32 + oy - r;
            draw_line_segment_mut(img, (x1 as f32, y1 as f32), (x2 as f32, y2 as f32), color);
        }
    }
}

async fn generate_pdf(
    deck: &DeckOrPile,
    db: &CardsDatabase,
    settings: ProxySheetSettings,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let (card_w, card_h) = settings.card_size.dimensions();
    let card_width_px: u32 = (DPI * INCH_PER_MM * card_w.0).ceil() as u32;
    let card_height_px: u32 = (DPI * INCH_PER_MM * card_h.0).ceil() as u32;

    let (page_width, page_height) = settings.paper_size.dimensions();

    let layout = Layout::compute(
        page_width,
        page_height,
        DPI,
        card_w,
        card_h,
        settings.gap,
        settings.crop_marks_position,
    );
    if layout.cards_per_page == 0 {
        return Err("Paper size is too small to fit any card with current margins/gap".into());
    }

    // Build the list of cards to print
    let cards: Box<dyn Iterator<Item = &crate::CommonCard>> = if settings.include_cheers {
        Box::new(deck.all_cards())
    } else {
        Box::new(
            deck.all_cards()
                .filter(|c| c.card_type(db) != Some(crate::CardType::Cheer)),
        )
    };
    let cards: Vec<_> = cards
        .filter(|c| {
            c.image_path(db, settings.card_lang, ImageOptions::proxy_print())
                .is_some()
        })
        .flat_map(|c| std::iter::repeat_n(c.clone(), c.amount as usize))
        .collect();

    let pages_count = (cards.len() as f32 / layout.cards_per_page as f32).ceil() as usize;

    // Create PDF document
    let title = format!("Proxy sheets for {}", deck.required_deck_name(db));
    let mut doc = PdfDocument::new(&title);
    doc.metadata.info.producer = "hololive OCG Deck Converter".to_string();
    // no metadata date for wasm, printpdf can't do it

    // Download and cache images once per unique card
    let img_cache = Arc::new(Mutex::new(HashMap::with_capacity(cards.len())));

    let download_tasks = deck.all_cards().map(|card| {
        let img_cache = img_cache.clone();
        async move {
            let Some(img_path) =
                card.image_path(db, settings.card_lang, ImageOptions::proxy_print())
            else {
                // Skip missing card
                return Ok::<(), Box<dyn Error>>(());
            };

            let image_bytes = reqwest::get(&img_path).await?.bytes().await?;
            let image = ::image::load_from_memory_with_format(&image_bytes, ImageFormat::WebP)?;
            let image = image.resize_exact(card_width_px, card_height_px, FilterType::CatmullRom);

            // Rotate the image if needed
            let (image, card_width_px, card_height_px) = if layout.rotated {
                (image.rotate90(), card_height_px, card_width_px)
            } else {
                (image, card_width_px, card_height_px)
            };

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
            let raw = RawImage::decode_from_bytes_async(&bytes.into_inner(), &mut vec![]).await?;

            let key = Some((&card.card_number, card.illustration_idx));
            img_cache.lock().await.insert(key, raw);

            Ok(())
        }
    });
    try_join_all(download_tasks).await?;

    let img_cache = Arc::try_unwrap(img_cache).unwrap().into_inner();

    // Add the images to the document resources and get their IDs
    let image_ids: HashMap<_, _> = img_cache
        .into_iter()
        .map(|(key, image)| (key, doc.add_image(&image)))
        .collect();

    // Optional crop marks overlay
    let overlay_id: Option<XObjectId> = if settings.crop_marks_size != CropMarksSize::None {
        let page_w_px = layout.page_width.into_pt().into_px(layout.dpi).0;
        let page_h_px = layout.page_height.into_pt().into_px(layout.dpi).0;
        let crop_mark_thickness = DEFAULT_CROP_MARK_THICKNESS
            .into_pt()
            .into_px(layout.dpi)
            .0
            .max(1) as u32;
        let crop_mark_color = ::image::Rgba([0x68, 0x68, 0x68, 0xFF]);
        let crop_mark_len = if let CropMarksSize::Mm(mark_size) = settings.crop_marks_size {
            Mm(mark_size).into_pt().into_px(layout.dpi).0 as u32
        } else if let CropMarksSize::Mm(default_size) = DEFAULT_CROP_MARK_SIZE {
            Mm(match layout.crop_marks_position {
                CropMarksPosition::Centered => default_size,
                CropMarksPosition::CardCorners => default_size * 2.0,
            })
            .into_pt()
            .into_px(layout.dpi)
            .0 as u32
        } else {
            unreachable!()
        };

        let mut overlay = ::image::RgbaImage::from_pixel(
            page_w_px as u32,
            page_h_px as u32,
            ::image::Rgba([0, 0, 0, 0]),
        );

        // Draw crop marks for each card slots
        let marks_positions = layout.crop_marks_positions();
        for (tx, ty, flags) in &marks_positions {
            let x_px = tx.into_pt().into_px(layout.dpi).0;
            let y_px = ty.into_pt().into_px(layout.dpi).0;

            flags.draw_crop_mark(
                &mut overlay,
                x_px as u32,
                y_px as u32,
                crop_mark_len,
                crop_mark_thickness,
                crop_mark_color,
            );
        }

        // Draw lines between crop marks for easier cutting with scissors
        if settings.crop_marks_size == CropMarksSize::FullLength {
            // Vertical lines
            let max_col = if layout.crop_marks_position == CropMarksPosition::Centered
                || layout.gap <= DEFAULT_CROP_MARK_THICKNESS
            {
                layout.fit_w + 1
            } else if layout.crop_marks_position == CropMarksPosition::CardCorners {
                layout.fit_w * 2
            } else {
                unreachable!()
            };
            for col in 0..max_col {
                let start = marks_positions[col];
                let end = marks_positions[marks_positions.len() - 1 - (max_col - 1 - col)];

                let x1_px = start.0.into_pt().into_px(layout.dpi).0;
                let y1_px = start.1.into_pt().into_px(layout.dpi).0;

                let x2_px = end.0.into_pt().into_px(layout.dpi).0;
                let y2_px = end.1.into_pt().into_px(layout.dpi).0;

                draw_line_thick_mut(
                    &mut overlay,
                    (x1_px as u32, y1_px as u32),
                    (x2_px as u32, y2_px as u32),
                    crop_mark_thickness,
                    crop_mark_color,
                );
            }

            // Horizontal lines
            let max_row = if layout.crop_marks_position == CropMarksPosition::Centered
                || layout.gap <= DEFAULT_CROP_MARK_THICKNESS
            {
                layout.fit_h + 1
            } else if layout.crop_marks_position == CropMarksPosition::CardCorners {
                layout.fit_h * 2
            } else {
                unreachable!()
            };
            for row in 0..max_row {
                let start = marks_positions[row * max_col];
                let end = marks_positions[row * max_col + max_col - 1];

                let x1_px = start.0.into_pt().into_px(layout.dpi).0;
                let y1_px = start.1.into_pt().into_px(layout.dpi).0;

                let x2_px = end.0.into_pt().into_px(layout.dpi).0;
                let y2_px = end.1.into_pt().into_px(layout.dpi).0;

                draw_line_thick_mut(
                    &mut overlay,
                    (x1_px as u32, y1_px as u32),
                    (x2_px as u32, y2_px as u32),
                    crop_mark_thickness,
                    crop_mark_color,
                );
            }
        }

        let mut overlay_bytes = Cursor::new(vec![]);
        ::image::DynamicImage::ImageRgba8(overlay)
            .write_to(&mut overlay_bytes, ImageFormat::Png)?;
        let overlay_raw =
            RawImage::decode_from_bytes_async(&overlay_bytes.into_inner(), &mut vec![]).await?;
        Some(doc.add_image(&overlay_raw))
    } else {
        None
    };

    // Build pages
    let pages = (0..pages_count)
        .map(|page_idx| {
            // Create operations for our page
            let mut ops = Vec::new();

            for idx_in_page in 0..layout.cards_per_page {
                let global_idx = page_idx * layout.cards_per_page + idx_in_page;
                let Some(card) = cards.get(global_idx) else {
                    break;
                };

                // Place the image on the page
                let key = Some((&card.card_number, card.illustration_idx));
                if let Some(image_id) = image_ids.get(&key) {
                    let (tx, ty) = layout.card_translate(idx_in_page);
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

            // Overlay crop marks above everything (optional)
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

            // Create a page with our operations
            PdfPage::new(page_width, page_height, ops)
        })
        .collect::<Vec<_>>();

    Ok(doc.with_pages(pages).save(
        &PdfSaveOptions {
            image_optimization: Some(ImageOptimizationOptions {
                // Don't resize, will lose image quality
                max_image_size: None,
                ..Default::default()
            }),
            ..Default::default()
        },
        &mut Vec::new(),
    ))
}

#[component]
pub fn Export(mut common_deck: Signal<DeckOrPile>, db: Signal<CardsDatabase>) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        language: CardLanguage,
        missing_proxies: bool,
        paper_size: PaperSize,
        include_cheers: bool,
        default_settings: bool,
        crop_marks_size: CropMarksSize,
        crop_marks_position: CropMarksPosition,
        card_size: CardSize,
        gap: Mm,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }
    impl TrackEvent for EventData {
        fn key(&self, event_name: &str) -> String {
            let data_str = serde_json::to_string(&(
                &self.format,
                &self.language,
                &self.missing_proxies,
                &self.paper_size,
                &self.include_cheers,
                &self.default_settings,
                &self.error,
            ))
            .unwrap_or_default();
            format!("{event_name}:{data_str}")
        }
    }

    let mut deck_error = use_signal(String::new);
    let card_lang = PREVIEW_CARD_LANG.signal();
    let mut paper_size = use_signal(|| match get_local_country().as_deref() {
        Some("US" | "CA" | "MX" | "CR" | "PA" | "DO" | "GT" | "CL" | "CO" | "VE" | "PE") => {
            PaperSize::Letter
        }
        _ => PaperSize::A4,
    });
    let mut include_cheers = use_signal(|| DEFAULT_INCLUDE_CHEERS);
    let mut crop_marks_size = use_signal(|| DEFAULT_CROP_MARK_SIZE);
    let mut crop_marks_position = use_signal(|| DEFAULT_CROP_MARK_POSITION);
    let mut card_size = use_signal(|| DEFAULT_CARD_SIZE);
    let mut gap = use_signal(|| DEFAULT_GAP);
    let mut loading = use_signal(|| false);
    let mut show_advanced = use_signal(|| false);

    let settings_count = use_memo(move || {
        [
            *crop_marks_size.read() != DEFAULT_CROP_MARK_SIZE,
            *crop_marks_position.read() != DEFAULT_CROP_MARK_POSITION,
            *card_size.read() != DEFAULT_CARD_SIZE,
            *gap.read() != DEFAULT_GAP,
        ]
        .iter()
        .filter(|&&x| x)
        .count()
    });

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
            PaperSize::Legal => "legal",
        };

        let file_name = common_deck.file_name(&db.read());
        let file_name = format!("{file_name}.proxy_sheets.{lang}_{ps}.pdf");

        let missing_proxies = has_missing_proxies(&common_deck, &db.read(), *card_lang.read());

        match generate_pdf(
            &common_deck,
            &db.read(),
            ProxySheetSettings {
                card_lang: *card_lang.read(),
                paper_size: *paper_size.read(),
                include_cheers: *include_cheers.read(),
                crop_marks_size: *crop_marks_size.read(),
                crop_marks_position: *crop_marks_position.read(),
                card_size: *card_size.read(),
                gap: *gap.read(),
            },
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
                        default_settings: *settings_count.read() == 0,
                        crop_marks_size: *crop_marks_size.read(),
                        crop_marks_position: *crop_marks_position.read(),
                        card_size: *card_size.read(),
                        gap: *gap.read(),
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
                        default_settings: *settings_count.read() == 0,
                        crop_marks_size: *crop_marks_size.read(),
                        crop_marks_position: *crop_marks_position.read(),
                        card_size: *card_size.read(),
                        gap: *gap.read(),
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
            allow_pile: true,
            card_lang,
            db,
            common_deck,
        }

        div { class: "block",
            div { class: "grid is-col-min-8",
                // Card language
                div { class: "cell",
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
                                option {
                                    selected: *PREVIEW_CARD_LANG.read() == CardLanguage::English,
                                    value: "en",
                                    "English"
                                }
                                option {
                                    selected: *PREVIEW_CARD_LANG.read() == CardLanguage::Japanese,
                                    value: "jp",
                                    "Japanese"
                                }
                            }
                        }
                    }
                }

                // Include cheers
                div { class: "cell",
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
                                option {
                                    selected: !*include_cheers.read(),
                                    value: "no",
                                    "No"
                                }
                                option {
                                    selected: *include_cheers.read(),
                                    value: "yes",
                                    "Yes"
                                }
                            }
                        }
                    }
                }

                // Paper size
                div { class: "cell",
                    label { "for": "paper_size", class: "label", "Paper size" }
                    div { class: "control",
                        div { class: "select",
                            select {
                                id: "paper_size",
                                oninput: move |ev| {
                                    *paper_size.write() = match ev.value().as_str() {
                                        "a4" => PaperSize::A4,
                                        "letter" => PaperSize::Letter,
                                        "legal" => PaperSize::Legal,
                                        _ => unreachable!(),
                                    };
                                },
                                option {
                                    selected: *paper_size.read() == PaperSize::A4,
                                    value: "a4",
                                    "A4 (210x297 mm)"
                                }
                                option {
                                    selected: *paper_size.read() == PaperSize::Letter,
                                    value: "letter",
                                    "Letter (8.5x11 in)"
                                }
                                option {
                                    selected: *paper_size.read() == PaperSize::Legal,
                                    value: "legal",
                                    "Legal (8.5x14 in)"
                                }
                            }
                        }
                    }
                }

            }
        }

        // Advanced settings
        div { class: if *show_advanced.read() { "field" } else { "block" },
            a {
                href: "#",
                role: "button",
                onclick: move |evt| {
                    evt.prevent_default();
                    let show = *show_advanced.read();
                    *show_advanced.write() = !show;
                },
                span { class: "icon",
                    i {
                        class: "fa-solid",
                        class: if *show_advanced.read() { "fa-chevron-down" } else { "fa-chevron-right" },
                    }
                }
                "Advanced settings"
                if *settings_count.read() > 0 {
                    " ({settings_count})"
                }
            }
        }

        if *show_advanced.read() {
            div { class: "block",
                div { class: "grid is-col-min-8",
                    // Crop marks
                    div { class: "cell",
                        label { "for": "include_crop_marks", class: "label", "Crop marks" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "include_crop_marks",
                                    oninput: move |ev| {
                                        *crop_marks_size.write() = match (
                                            ev.value().as_str(),
                                            ev.value().parse::<f32>(),
                                        ) {
                                            ("none", _) => CropMarksSize::None,
                                            ("full", _) => CropMarksSize::FullLength,
                                            (_, Ok(val)) => CropMarksSize::Mm(val),
                                            _ => unreachable!(),
                                        };
                                    },
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::None,
                                        value: "none",
                                        "None"
                                    }
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::Mm(3.0),
                                        value: "3",
                                        "3mm"
                                    }
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::Mm(5.0),
                                        value: "5",
                                        "5mm"
                                    }
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::Mm(10.0),
                                        value: "10",
                                        "10mm"
                                    }
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::Mm(20.0),
                                        value: "20",
                                        "20mm"
                                    }
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::Mm(30.0),
                                        value: "30",
                                        "30mm"
                                    }
                                    option {
                                        selected: *crop_marks_size.read() == CropMarksSize::FullLength,
                                        value: "full",
                                        "Full length"
                                    }
                                }
                            }
                        }
                    }

                    // Crop marks position
                    div { class: "cell",
                        label { "for": "crop_marks_position", class: "label", "Crop marks position" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "crop_marks_position",
                                    disabled: *crop_marks_size.read() == CropMarksSize::None,
                                    oninput: move |ev| {
                                        *crop_marks_position.write() = match ev.value().as_str() {
                                            "centered" => CropMarksPosition::Centered,
                                            "card_corners" => CropMarksPosition::CardCorners,
                                            _ => unreachable!(),
                                        };
                                    },
                                    option {
                                        selected: *crop_marks_position.read() == CropMarksPosition::Centered,
                                        value: "centered",
                                        "Centered"
                                    }
                                    option {
                                        selected: *crop_marks_position.read() == CropMarksPosition::CardCorners,
                                        value: "card_corners",
                                        "Card corners"
                                    }
                                }
                            }
                        }
                    }

                    // Card size
                    div { class: "cell",
                        label { "for": "card_size", class: "label", "Card size" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    id: "card_size",
                                    oninput: move |ev| {
                                        *card_size.write() = match ev.value().as_str() {
                                            "metric" => CardSize::Metric,
                                            "imperial" => CardSize::Imperial,
                                            _ => unreachable!(),
                                        };
                                    },
                                    option {
                                        selected: *card_size.read() == CardSize::Metric,
                                        value: "metric",
                                        "Metric (63x88 mm)"
                                    }
                                    option {
                                        selected: *card_size.read() == CardSize::Imperial,
                                        value: "imperial",
                                        "Imperial (2.5x3.5 in)"
                                    }
                                }
                            }
                        }
                    }

                    // Gap
                    div { class: "cell",
                        label { "for": "gap", class: "label", "Gap between cards (mm)" }
                        div { class: "control",
                            input {
                                id: "gap",
                                r#type: "number",
                                class: "input",
                                style: "width: auto;",
                                min: "0",
                                max: "10",
                                step: "0.5",
                                maxlength: "4",
                                value: gap.read().0.to_string(),
                                oninput: move |ev| {
                                    if let Ok(val) = ev.value().parse::<f32>() {
                                        *gap.write() = Mm(val.max(0.0));
                                    }
                                },
                            }
                        }
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
