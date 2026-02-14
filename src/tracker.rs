use jiff::{SignedDuration, Timestamp};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use dioxus::{logger::tracing::debug, prelude::spawn};
use gloo::utils::{document, window};
use reqwest::{Client, ClientBuilder};
use serde::Serialize;
use serde_json::{Value, json};

use crate::HOCG_DECK_CONVERT_API;
use crate::VERSION;

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(|| ClientBuilder::new().build().unwrap())
}

// Global map to track timestamps for throttling
fn event_timestamps() -> &'static Mutex<HashMap<String, Timestamp>> {
    static EVENT_TIMESTAMPS: OnceLock<Mutex<HashMap<String, Timestamp>>> = OnceLock::new();
    EVENT_TIMESTAMPS.get_or_init(|| Mutex::new(HashMap::new()))
}

// Default throttling duration
const THROTTLE_DURATION: SignedDuration = SignedDuration::from_mins(1); // 1 minute default

pub enum EventType {
    Entry,
    Import(String),
    Export(String),
    EditDeck,
    Url(String),
    Error,
}

// Generate a key for the throttling map based on event type and data
fn generate_event_key<T: Serialize>(event_name: &str, data: &T) -> String {
    let data_str = serde_json::to_string(data).unwrap_or_default();
    format!("{event_name}:{data_str}")
}

pub fn track_event<T>(event: EventType, data: T)
where
    T: serde::ser::Serialize,
{
    let is_entry = matches!(event, EventType::Entry);

    let event = match event {
        EventType::Entry => Some("$pageview"),
        EventType::Import(_fmt) => Some("import"),
        EventType::Export(_fmt) => Some("export"),
        EventType::EditDeck => Some("edit_deck"),
        EventType::Url(_url) => Some("external_url"),
        EventType::Error => Some("error"),
    };

    // Check throttling for events that have a name
    if let Some(event) = event {
        let event_key = generate_event_key(event, &data);

        // Check if this event has been tracked recently
        let mut timestamps = event_timestamps().lock().unwrap();
        let now = Timestamp::now();

        if let Some(last_time) = timestamps.get(&event_key)
            && now.duration_since(*last_time) < THROTTLE_DURATION
        {
            // Too soon, don't track
            return;
        }

        // Update the timestamp
        timestamps.insert(event_key, now);
    }

    // https://posthog.com/docs/data/events#default-properties
    let mut properties = json!({
        "$current_url": window().location().href().ok(),
        "$host": window().location().hostname().ok(),
        "$pathname": window().location().pathname().ok(),
        "$raw_user_agent": window().navigator().user_agent().ok(),
        "$user_agent": window().navigator().user_agent().ok(),
        "$referrer": document().referrer(),
        "$screen_height": window().screen().and_then(|s| s.height()).ok(),
        "$screen_width": window().screen().and_then(|s| s.width()).ok(),
        "$session_id": session_id(),
    });

    if let Value::Object(properties) = &mut properties {
        // append standalone mode info for entry event, which is used to track PWA usage
        if is_entry {
            let standalone_display_mode = window()
                .match_media("(display-mode: standalone)")
                .ok()
                .flatten()
                .map(|mq| mq.matches())
                .unwrap_or(false);

            let android_app_referrer = document().referrer().starts_with("android-app://");
            let is_standalone = standalone_display_mode || android_app_referrer;

            properties.insert("is_standalone".into(), is_standalone.into());
            properties.insert(
                "standalone_display_mode".into(),
                standalone_display_mode.into(),
            );
            properties.insert("android_app_referrer".into(), android_app_referrer.into());
        }

        // insert version into properties
        properties.insert("version".into(), VERSION.into());
        // append event data into properties
        let mut data = json!(data);
        if let Value::Object(data) = &mut data {
            properties.append(data);
        }
    }

    let request = json!({
        "event": event.unwrap_or("$pageview"),
        "distinct_id": distinct_id(),
        "properties": properties,
        "timestamp": Timestamp::now(),
    });
    debug!("{request:?}");

    // skip tracking
    if no_tracking() {
        return;
    }

    // we as few await point as possible, so we are sending the request in a new task
    spawn(async move {
        // we don't care about any errors
        let _resp = http_client()
            .post(format!("{HOCG_DECK_CONVERT_API}/posthog"))
            .json(&request)
            .send()
            .await;
    });
}

pub fn track_url(title: &str) {
    #[derive(Serialize)]
    struct EventData<'a> {
        title: &'a str,
    }

    track_event(EventType::Url(title.into()), EventData { title });
}

pub fn track_error(message: &str) {
    #[derive(Serialize)]
    struct EventData<'a> {
        message: &'a str,
    }

    track_event(EventType::Error, EventData { message });
}

fn no_tracking() -> bool {
    let Some(ls) = window().local_storage().ok().flatten() else {
        return false;
    };

    ls.get_item("umami.disabled").ok().flatten().is_some()
        || ls.get_item("posthog.disabled").ok().flatten().is_some()
}

fn distinct_id() -> Option<String> {
    const DISTINCT_ID_KEY: &str = "distinct_id";

    // get distinct id from local storage
    let ls = window().local_storage().ok().flatten()?;

    if let Some(distinct_id) = ls.get_item(DISTINCT_ID_KEY).ok().flatten() {
        Some(distinct_id)
    } else {
        // if not exist, generate a new one and store it
        let new_distinct_id = uuid::Uuid::new_v4().to_string();
        ls.set_item(DISTINCT_ID_KEY, &new_distinct_id).ok()?;
        Some(new_distinct_id)
    }
}

fn session_id() -> Option<String> {
    const SESSION_ID_KEY: &str = "session_id";

    // get session id from session storage
    let ss = window().session_storage().ok().flatten()?;

    if let Some(session_id) = ss.get_item(SESSION_ID_KEY).ok().flatten() {
        Some(session_id)
    } else {
        // if not exist, generate a new one and store it
        // https://posthog.com/docs/data/sessions#custom-session-ids
        let new_session_id = uuid::Uuid::now_v7().to_string();
        ss.set_item(SESSION_ID_KEY, &new_session_id).ok()?;
        Some(new_session_id)
    }
}
