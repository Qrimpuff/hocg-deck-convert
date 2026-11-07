use std::{collections::BTreeMap, error::Error, ops::Not};

use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::DeckType;
use crate::sources::DeckOrPile;

use super::deck_file::{DeckFileExport, DeckFileImport};
use super::{CardsDatabase, CommonCard, CommonDeck, MergeCommonCards};

const FORMAT_PREFIX: &str = "hocg1.";

// card number -> art key, e.g. "hBP02-033" -> "SR_2"
type ArtKeys = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OshiCard(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckCard(String, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckArt(String, String);

fn to_deck_cards(cards: Vec<CommonCard>) -> Vec<DeckCard> {
    cards
        .merge()
        .into_iter()
        .map(|c| DeckCard(c.card_number, c.amount))
        .collect()
}

fn to_common_cards(cards: Vec<DeckCard>, arts: &ArtKeys, db: &CardsDatabase) -> Vec<CommonCard> {
    cards
        .into_iter()
        .map(|DeckCard(card_number, amount)| {
            CommonCard::from_card_number_and_holoburi_art_key(
                card_number.clone(),
                arts.get(&card_number).map(String::as_str),
                amount,
                db,
            )
        })
        .collect::<Vec<_>>()
        .merge()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    #[serde(rename = "n", skip_serializing_if = "Option::is_none")]
    deck_name: Option<String>,

    #[serde(rename = "o")]
    oshi: OshiCard,

    #[serde(rename = "m")]
    deck: Vec<DeckCard>,

    #[serde(rename = "c")]
    cheer_deck: Vec<DeckCard>,

    #[serde(rename = "a", default, skip_serializing_if = "Vec::is_empty")]
    arts: Vec<DeckArt>,
}

impl Deck {
    pub fn from_file(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Self::from_text(std::str::from_utf8(bytes)?)
    }

    // Accepts unpadded or padded base64url, as holoBuri may emit either.
    pub fn from_text(text: &str) -> Result<Self, Box<dyn Error>> {
        let encoded = text
            .strip_prefix(FORMAT_PREFIX)
            .ok_or("Invalid deck format prefix")?;

        let decoded = BASE64_URL_SAFE_NO_PAD.decode(encoded.trim_end_matches('='))?;
        Ok(serde_json::from_slice(&decoded)?)
    }

    pub fn to_file(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self.to_text()?.into_bytes())
    }

    // Produces hocg1.BASE64URL(compact JSON), without padding.
    pub fn to_text(&self) -> Result<String, Box<dyn Error>> {
        let json = serde_json::to_vec(self)?;
        Ok(format!(
            "{FORMAT_PREFIX}{}",
            BASE64_URL_SAFE_NO_PAD.encode(json)
        ))
    }

    pub fn from_common_deck(deck: CommonDeck, db: &CardsDatabase) -> Option<Self> {
        let oshi = deck.oshi?;

        let mut arts = ArtKeys::new();
        for card in [&oshi]
            .into_iter()
            .chain(&deck.main_deck)
            .chain(&deck.cheer_deck)
        {
            if let Some(art_key) = card
                .card_illustration(db)
                .and_then(|i| i.holoburi_art_key.clone())
            {
                arts.entry(card.card_number.clone()).or_insert(art_key);
            }
        }

        Some(Deck {
            deck_name: deck.name,
            oshi: OshiCard(oshi.card_number),
            deck: to_deck_cards(deck.main_deck),
            cheer_deck: to_deck_cards(deck.cheer_deck),
            arts: arts
                .into_iter()
                .map(|(card_number, art_key)| DeckArt(card_number, art_key))
                .collect(),
        })
    }

    pub fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck {
        let arts: ArtKeys = value
            .arts
            .into_iter()
            .map(|DeckArt(card_number, art_key)| (card_number, art_key))
            .collect();

        CommonDeck {
            name: value
                .deck_name
                .and_then(|n| n.trim().is_empty().not().then_some(n)),
            oshi: Some(CommonCard::from_card_number_and_holoburi_art_key(
                value.oshi.0.clone(),
                arts.get(&value.oshi.0).map(String::as_str),
                1,
                db,
            )),
            main_deck: to_common_cards(value.deck, &arts, db),
            cheer_deck: to_common_cards(value.cheer_deck, &arts, db),
        }
    }
}

#[component]
pub fn Import(mut common_deck: Signal<DeckOrPile>, db: Signal<CardsDatabase>) -> Element {
    rsx! {
        DeckFileImport {
            deck_type: DeckType::HoloBuri,
            fallback_deck_type: DeckType::HoloBuri,
            import_name: "holoBuri",
            file_extension: "txt",
            format_label: "deck code",
            common_deck,
            db,
        }
    }
}

#[component]
pub fn Export(mut common_deck: Signal<DeckOrPile>, db: Signal<CardsDatabase>) -> Element {
    rsx! {
        DeckFileExport {
            deck_type: DeckType::HoloBuri,
            export_name: "holoBuri",
            export_id: "holoburi",
            file_extension: "txt",
            format_label: "deck code",
            allow_unreleased: true,
            common_deck,
            db,
        }
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_URL_SAFE;

    use super::*;

    // hocg1.BASE64URL(JSON) with art key, unpadded
    const CODE_WITH_ARTS: &str = "hocg1.eyJuIjoiVGVzdCA_PiIsIm8iOiJoWTAxLTAwMSIsIm0iOltbImhCUDAyLTAzMyIsMl1dLCJjIjpbWyJoQlAwMS0wNTAiLDFdXSwiYSI6W1siaEJQMDItMDMzIiwiU1JfMiJdXX0";
    // hocg1.BASE64URL(JSON) from before art keys existed
    const CODE_WITHOUT_ARTS: &str =
        "hocg1.eyJvIjoiaFkwMS0wMDEiLCJtIjpbWyJoQlAwMS0wNTAiLDFdXSwiYyI6W119";

    #[test]
    fn parses_unpadded_code_with_arts() {
        let deck = Deck::from_text(CODE_WITH_ARTS).unwrap();
        assert_eq!(deck.deck_name.as_deref(), Some("Test ?>"));
        assert_eq!(deck.oshi.0, "hY01-001");
        assert_eq!(deck.deck[0].0, "hBP02-033");
        assert_eq!(deck.deck[0].1, 2);
        assert_eq!(deck.arts[0].0, "hBP02-033");
        assert_eq!(deck.arts[0].1, "SR_2");
    }

    #[test]
    fn parses_padded_code() {
        let json = BASE64_URL_SAFE_NO_PAD
            .decode(&CODE_WITH_ARTS[FORMAT_PREFIX.len()..])
            .unwrap();
        let padded = format!("{FORMAT_PREFIX}{}", BASE64_URL_SAFE.encode(json));
        let deck = Deck::from_text(&padded).unwrap();
        assert_eq!(deck.arts.len(), 1);
    }

    #[test]
    fn parses_code_without_arts() {
        let deck = Deck::from_text(CODE_WITHOUT_ARTS).unwrap();
        assert_eq!(deck.oshi.0, "hY01-001");
        assert!(deck.arts.is_empty());
    }

    #[test]
    fn round_trips_and_omits_empty_arts() {
        let deck = Deck::from_text(CODE_WITHOUT_ARTS).unwrap();
        let text = deck.to_text().unwrap();
        assert_eq!(text, CODE_WITHOUT_ARTS);
        assert!(!text.contains('='));

        let deck = Deck::from_text(CODE_WITH_ARTS).unwrap();
        assert_eq!(deck.to_text().unwrap(), CODE_WITH_ARTS);
    }
}
