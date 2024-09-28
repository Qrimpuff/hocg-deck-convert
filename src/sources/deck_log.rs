use std::error::Error;
use std::sync::OnceLock;

use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};

use super::{CardsInfoMap, CommonCards, CommonCardsConversion, CommonDeck, CommonDeckConversion};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Cards {
    card_number: String,
    num: u32,
    manage_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Deck {
    game_title_id: u32,
    deck_id: String,
    title: String,
    p_list: Vec<Cards>,   // oshi
    list: Vec<Cards>,     // main deck
    sub_list: Vec<Cards>, // cheer deck
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
            .post("https://hocg-deck-log-proxy.shuttleapp.rs/view-deck")
            .json(&req)
            .send()
            .await
            .unwrap();

        let content = resp.text().await.unwrap();
        info!("{:?}", content);

        Ok(serde_json::from_str(&content).map_err(|_| content)?)
    }

    pub async fn publish(&self) -> Result<String, Box<dyn Error>> {
        todo!()
    }
}

impl CommonCardsConversion for Cards {
    fn from_common_cards(cards: CommonCards, _map: &CardsInfoMap) -> Self {
        Cards {
            card_number: cards.card_number,
            num: cards.amount,
            manage_id: cards.manage_id.expect("should be a valid card in deck log"),
        }
    }

    fn to_common_cards(value: Self, _map: &CardsInfoMap) -> CommonCards {
        CommonCards {
            manage_id: Some(value.manage_id),
            card_number: value.card_number,
            amount: value.num,
        }
    }
}

impl CommonDeckConversion for Deck {
    fn from_common_deck(deck: CommonDeck, map: &CardsInfoMap) -> Self {
        Deck {
            game_title_id: 9,       // 9 for hOCG on JP deck log // TODO adjust for EN
            deck_id: "TODO".into(), // TODO not sure what to put here?
            title: deck.required_deck_name(),
            p_list: vec![Cards::from_common_cards(deck.oshi, map)],
            list: deck
                .main_deck
                .into_iter()
                .map(|c| Cards::from_common_cards(c, map))
                .collect(),
            sub_list: deck
                .cheer_deck
                .into_iter()
                .map(|c| Cards::from_common_cards(c, map))
                .collect(),
        }
    }

    fn to_common_deck(mut value: Self, map: &CardsInfoMap) -> CommonDeck {
        CommonDeck {
            name: Some(value.title),
            oshi: Cards::to_common_cards(value.p_list.swap_remove(0), map),
            main_deck: value
                .list
                .into_iter()
                .map(|c| Cards::to_common_cards(c, map))
                .collect(),
            cheer_deck: value
                .sub_list
                .into_iter()
                .map(|c| Cards::to_common_cards(c, map))
                .collect(),
        }
    }
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

#[component]
pub fn Import(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
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

        info!("{:?}", deck);
        match deck {
            Ok(deck) => {
                *deck_log_url.write() = deck.view_url();
                *common_deck.write() = Some(Deck::to_common_deck(deck, &map.read()));
            }
            Err(e) => *deck_error.write() = e.to_string(),
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
                    value: "{import_url_code}"
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
                        i { class: "fa-solid fa-download" }
                    }
                    span { "Import deck" }
                }
            }
        }
    }
}

// #[component]
// pub fn Export(mut common_deck: Signal<Option<CommonDeck>>, map: Signal<CardsInfoMap>) -> Element {
//     let mut deck_error = use_signal(String::new);

//     let deck: Option<Deck> = common_deck
//         .read()
//         .as_ref()
//         .map(|d| Deck::from_common_deck(d.clone(), &map.read()));
//     info!("{:?}", deck);
//     let text = match deck {
//         Some(deck) => match deck.to_text() {
//             Ok(text) => text,
//             Err(e) => {
//                 *deck_error.write() = e.to_string();
//                 "".into()
//             }
//         },
//         None => "".into(),
//     };

//     let download_file = move |_| {
//         let deck: Option<_> = common_deck.read().as_ref().map(|d| {
//             (
//                 d.file_name(),
//                 Deck::from_common_deck(d.clone(), &map.read()),
//             )
//         });
//         if let Some((file_name, deck)) = deck {
//             let file_name = format!("{}.holodelta.json", file_name.unwrap_or("deck".into()));
//             match deck.to_file() {
//                 Ok(file) => download_file(&file_name, &file[..]),
//                 Err(e) => *deck_error.write() = e.to_string(),
//             }
//         }
//     };

//     rsx! {
//         div { class: "field",
//             div { class: "control",
//                 button {
//                     class: "button",
//                     disabled: common_deck.read().is_none(),
//                     r#type: "button",
//                     onclick: download_file,
//                     span { class: "icon",
//                         i { class: "fa-solid fa-download" }
//                     }
//                     span { "Download deck file" }
//                 }
//             }
//         }
//         div { class: "field",
//             label { "for": "holodelta_export_json", class: "label", "holoDelta json" }
//             div { class: "control",
//                 textarea {
//                     id: "holodelta_export_json",
//                     class: "textarea",
//                     readonly: true,
//                     value: "{text}"
//                 }
//             }
//             p { class: "help is-danger", "{deck_error}" }
//         }
//     }
// }
