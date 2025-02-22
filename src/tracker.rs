use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::spawn};
use gloo::utils::{document, window};
use reqwest::{Client, ClientBuilder};
use serde::Serialize;
use serde_json::{Value, json};

use crate::HOCG_DECK_CONVERT_API;

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

pub enum EventType {
    Entry,
    Import(String),
    Export(String),
    Url(String),
}

pub fn track_event<T>(event: EventType, data: T)
where
    T: serde::ser::Serialize,
{
    let event = match event {
        // entry event doesn't have a name
        EventType::Entry => None,
        EventType::Import(_fmt) => Some("import"),
        EventType::Export(_fmt) => Some("export"),
        EventType::Url(_url) => Some("external_url"),
    };

    let mut payload = json!({
      "payload": {
        "hostname": window().location().hostname().ok(),
        "language": window().navigator().language(),
        "referrer": document().referrer(),
        "screen": window().screen().and_then(|s| Ok(format!("{}x{}", s.width()?, s.height()?))).ok(),
        "title": document().title(),
        "url": window().location().pathname().ok(),
        // website-id for hololive OCG Deck Converter
        "website": "eaaa2375-48a2-47cc-8d62-88a633825515",
      },
      "type": "event"
    });
    if let Value::Object(payload) = &mut payload {
        if let Some(Value::Object(payload)) = payload.get_mut("payload") {
            if let Some(event) = event {
                payload.insert("name".into(), event.into());
                payload.insert("data".into(), json!(data));
            }
        }
    }

    debug!("{payload:?}");

    // skip tracking
    let untrack = window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|ls| ls.get_item("umami.disabled").ok())
        .flatten()
        .is_some();
    if untrack {
        return;
    }

    // we as few await point as possible, so we are sending the request in a new task
    spawn(async move {
        let _resp = http_client()
            .post(format!("{HOCG_DECK_CONVERT_API}/umami"))
            .json(&payload)
            .send()
            .await
            .unwrap();
    });
}

pub fn track_url(title: &str) {
    #[derive(Serialize)]
    struct EventData<'a> {
        title: &'a str,
    }

    track_event(EventType::Url(title.into()), EventData { title });
}
