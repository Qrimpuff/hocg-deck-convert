use std::sync::OnceLock;
use std::{collections::HashMap, error::Error};

use dioxus::logger::tracing::debug;
use dioxus::prelude::*;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};

use crate::{EventType, HOCG_DECK_CONVERT_API, track_event};

use super::{
    CardsDatabase, CommonCard, CommonCardConversion, CommonDeck, CommonDeckConversion, MergeCommonCards,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Card {
    card_number: String,
    num: u32,
    manage_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Deck {
    game_title_id: u32,
    deck_id: String,
    title: String,
    p_list: Vec<Card>,   // oshi
    list: Vec<Card>,     // main deck
    sub_list: Vec<Card>, // cheer deck
}

impl Deck {
    fn view_url(&self) -> String {
        let base_url = match self.game_title_id {
            8 => "https://decklog-en.bushiroad.com/view",
            108 => "https://decklog-en.bushiroad.com/ja/view",
            9 => "https://decklog.bushiroad.com/view",
            _ => unreachable!("not valid game_title_id: {}", self.game_title_id),
        };

        format!("{base_url}/{}", self.deck_id)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ViewDeckRequest {
    game_title_id: Option<u32>,
    code: String,
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
struct PublishDeckRequest(Deck);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ViewDeckResult {
    deck_id: String,
}

impl Deck {
    pub async fn from_url(url: &str) -> Result<Self, Box<dyn Error>> {
        let url = url.trim().to_lowercase();
        let (game_title_id, code) = if url.starts_with("https://decklog-en.bushiroad.com/view/") {
            (8, url.replace("https://decklog-en.bushiroad.com/view/", ""))
        } else if url.starts_with("https://decklog-en.bushiroad.com/ja/view/") {
            (
                108,
                url.replace("https://decklog-en.bushiroad.com/ja/view/", ""),
            )
        } else if url.starts_with("https://decklog.bushiroad.com/view/") {
            (9, url.replace("https://decklog.bushiroad.com/view/", ""))
        } else {
            return Err("invalid url".into());
        };

        if !code.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')) {
            return Err("invalid code".into());
        }

        Deck::from_code(Some(game_title_id), &code).await
    }

    pub async fn from_code(game_title_id: Option<u32>, code: &str) -> Result<Self, Box<dyn Error>> {
        let req = ViewDeckRequest {
            game_title_id,
            code: code.into(),
        };

        let resp = http_client()
            .post(format!("{HOCG_DECK_CONVERT_API}/view-deck"))
            .json(&req)
            .send()
            .await
            .unwrap();

        let content = resp.text().await.unwrap();
        debug!("{:?}", content);

        Ok(serde_json::from_str(&content).map_err(|_| content)?)
    }

    pub async fn publish(&mut self, game_title_id: u32) -> Result<String, Box<dyn Error>> {
        let mut req = PublishDeckRequest(self.clone());
        req.0.game_title_id = game_title_id;

        let resp = http_client()
            .post(format!("{HOCG_DECK_CONVERT_API}/publish-deck"))
            .json(&req)
            .send()
            .await
            .unwrap();

        let content = resp.text().await.unwrap();
        debug!("{:?}", content);

        let res: ViewDeckResult = serde_json::from_str(&content).map_err(|_| content)?;

        self.game_title_id = game_title_id;
        self.deck_id = res.deck_id;

        Ok(self.view_url())
    }
}

impl CommonCardConversion for Card {
    type CardDeck = Vec<Card>;

    fn from_common_card(card: CommonCard, _db: &CardsDatabase) -> Self {
        Card {
            card_number: card.card_number,
            num: card.amount,
            manage_id: card
                .manage_id
                .expect("should be a valid card in deck log")
                .to_string(),
        }
    }

    fn to_common_card(value: Self, _db: &CardsDatabase) -> CommonCard {
        CommonCard {
            manage_id: value.manage_id.parse().ok(),
            card_number: value.card_number,
            amount: value.num,
        }
    }

    fn build_custom_deck(cards: Vec<CommonCard>, db: &CardsDatabase) -> Self::CardDeck {
        cards
            .merge()
            .into_iter()
            .map(|c| Card::from_common_card(c, db))
            .collect()
    }

    fn build_common_deck(cards: Self::CardDeck, db: &CardsDatabase) -> Vec<CommonCard> {
        cards
            .into_iter()
            .map(|c| Card::to_common_card(c, db))
            .collect::<Vec<_>>()
            .merge()
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, db: &CardsDatabase) -> Option<Self> {
        Some(Deck {
            game_title_id: 0,   // is set before publishing
            deck_id: "".into(), // not used for publishing
            title: deck.required_deck_name(),
            p_list: Card::build_custom_deck(deck.oshi.into_iter().collect(), db),
            list: Card::build_custom_deck(deck.main_deck, db),
            sub_list: Card::build_custom_deck(deck.cheer_deck, db),
        })
    }

    fn to_common_deck(value: Self, db: &CardsDatabase) -> CommonDeck {
        CommonDeck {
            name: Some(value.title),
            oshi: Some(Card::build_common_deck(value.p_list, db).swap_remove(0)),
            main_deck: Card::build_common_deck(value.list, db),
            cheer_deck: Card::build_common_deck(value.sub_list, db),
        }
    }
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

#[component]
pub fn Import(
    mut common_deck: Signal<CommonDeck>,
    db: Signal<CardsDatabase>,
    show_price: Signal<bool>,
) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        game_title_id: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        deck_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let mut deck_error = use_signal(String::new);
    let mut import_url_code = use_signal(String::new);
    let mut is_url = use_signal(|| false);
    let mut deck_log_url = use_signal(String::new);
    let mut loading = use_signal(|| false);

    // https://decklog.bushiroad.com/view/6ADJR

    let update_url_code = move |event: Event<FormData>| {
        let url_code = event.value();

        *import_url_code.write() = url_code.clone();
        *deck_error.write() = "".into();
        *deck_log_url.write() = String::new();

        let url_code = url_code.trim().to_lowercase();
        // url check
        let code = if url_code.starts_with("https://decklog-en.bushiroad.com/view/") {
            *is_url.write() = true;
            url_code.replace("https://decklog-en.bushiroad.com/view/", "")
        } else if url_code.starts_with("https://decklog-en.bushiroad.com/ja/view/") {
            *is_url.write() = true;
            url_code.replace("https://decklog-en.bushiroad.com/ja/view/", "")
        } else if url_code.starts_with("https://decklog.bushiroad.com/view/") {
            *is_url.write() = true;
            url_code.replace("https://decklog.bushiroad.com/view/", "")
        } else {
            *is_url.write() = false;
            url_code
        };

        // code check
        if !code.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')) {
            *deck_error.write() = "Invalid code".into();
        }
    };

    let import_deck = move |_| async move {
        *loading.write() = true;
        *deck_log_url.write() = String::new();

        let deck = if *is_url.read() {
            Deck::from_url(&import_url_code.read()).await
        } else {
            Deck::from_code(None, &import_url_code.read()).await
        };

        debug!("{:?}", deck);
        match deck {
            Ok(deck) => {
                *deck_log_url.write() = deck.view_url();
                track_event(
                    EventType::Import("Deck Log".into()),
                    EventData {
                        format: "Deck Log",
                        game_title_id: Some(deck.game_title_id),
                        deck_id: Some(deck.deck_id.clone()),
                        error: None,
                    },
                );
                *common_deck.write() = Deck::to_common_deck(deck, &db.read());
                *show_price.write() = false;
            }
            Err(e) => {
                *deck_error.write() = e.to_string();
                track_event(
                    EventType::Import("Deck Log".into()),
                    EventData {
                        format: "Deck Log",
                        game_title_id: None,
                        deck_id: None,
                        error: Some(e.to_string()),
                    },
                );
            }
        }

        *loading.write() = false;
    };

    rsx! {
        div { class: "field",
            label { "for": "deck_log_import_url_code", class: "label", "Deck Log URL or code" }
            div { class: "control",
                input {
                    id: "deck_log_import_url_code",
                    class: "input",
                    disabled: *loading.read(),
                    r#type: "text",
                    placeholder: "https://decklog.bushiroad.com/view/....",
                    oninput: update_url_code,
                    value: "{import_url_code}",
                }
            }
            p { class: "help is-danger", "{deck_error}" }
            if !deck_log_url.read().is_empty() {
                p { class: "help",
                    a { href: "{deck_log_url}", target: "_blank", "{deck_log_url}" }
                }
            }
        }

        div { class: "field",
            div { class: "control",
                button {
                    r#type: "button",
                    class: "button",
                    class: if *loading.read() { "is-loading" },
                    disabled: import_url_code.read().is_empty() || !deck_error.read().is_empty()
                        || *loading.read(),
                    onclick: import_deck,
                    span { class: "icon",
                        i { class: "fa-solid fa-cloud-arrow-down" }
                    }
                    span { "Import deck" }
                }
            }
        }
    }
}

static PUBLISH_CACHE: GlobalSignal<HashMap<(u32, u64), String>> = Signal::global(Default::default);

#[component]
pub fn Export(mut common_deck: Signal<CommonDeck>, db: Signal<CardsDatabase>) -> Element {
    #[derive(Serialize)]
    struct EventData {
        format: &'static str,
        game_title_id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        deck_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let mut deck_error = use_signal(String::new);
    let mut game_title_id = use_signal(|| 9); // default to Deck Log JP
    let mut deck_log_url = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let warnings = common_deck.read().validate(&db.read());

    let publish_deck = move |_| async move {
        let common_deck = common_deck.read();
        *loading.write() = true;
        *deck_log_url.write() = String::new();
        *deck_error.write() = String::new();

        if let Some(url) = PUBLISH_CACHE
            .read()
            .get(&(*game_title_id.read(), common_deck.calculate_hash()))
        {
            *deck_log_url.write() = url.clone();
            *loading.write() = false;
            return;
        }

        let deck = Deck::from_common_deck(common_deck.clone(), &db.read());
        if let Some(mut deck) = deck {
            match deck.publish(*game_title_id.read()).await {
                Ok(url) => {
                    *deck_log_url.write() = url.clone();
                    PUBLISH_CACHE
                        .write()
                        .insert((*game_title_id.read(), common_deck.calculate_hash()), url);
                    track_event(
                        EventType::Export("Deck Log".into()),
                        EventData {
                            format: "Deck Log",
                            game_title_id: *game_title_id.read(),
                            deck_id: Some(deck.deck_id.clone()),
                            error: None,
                        },
                    );
                }
                Err(e) => {
                    *deck_error.write() = e.to_string();
                    track_event(
                        EventType::Export("Deck Log".into()),
                        EventData {
                            format: "Deck Log",
                            game_title_id: *game_title_id.read(),
                            deck_id: None,
                            error: Some(e.to_string()),
                        },
                    );
                }
            }
        }

        *loading.write() = false;
    };

    rsx! {
        div { class: "field",
            label { "for": "game_title_id", class: "label", "Deck Log language" }
            div { class: "control",
                div { class: "select",
                    select {
                        id: "game_title_id",
                        oninput: move |ev| {
                            *deck_log_url.write() = String::new();
                            *deck_error.write() = String::new();
                            *game_title_id.write() = match ev.value().as_str() {
                                "9" => 9,
                                "108" => 108,
                                _ => unreachable!(),
                            };
                        },
                        option { value: "9", "Deck Log JP" }
                        option { value: "108", "Deck Log EN (JP version)" }
                    }
                }
            }
            p { class: "help is-danger", "{deck_error}" }
            if !deck_log_url.read().is_empty() {
                p { class: "help",
                    a { href: "{deck_log_url}", target: "_blank", "{deck_log_url}" }
                }
            }
        }

        div { class: "field",
            div { class: "control",
                button {
                    r#type: "button",
                    class: "button",
                    class: if *loading.read() { "is-loading" },
                    disabled: !warnings.is_empty() || *loading.read(),
                    onclick: publish_deck,
                    span { class: "icon",
                        i { class: "fa-solid fa-cloud-arrow-up" }
                    }
                    span { "Publish deck" }
                }
            }
        }
    }
}
