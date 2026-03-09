use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Cursor {
    pub timestamp: DateTime<Utc>,
    pub id: Uuid,
}

impl Cursor {
    pub fn new(timestamp: DateTime<Utc>, id: Uuid) -> Self {
        Self { timestamp, id }
    }

    pub fn encode(&self) -> String {
        let raw = format!(
            "{}:{}",
            self.timestamp.timestamp_nanos_opt().unwrap_or(0),
            self.id
        );
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    pub fn decode(encoded: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
        let raw = String::from_utf8(bytes).ok()?;
        let (ts_str, id_str) = raw.split_once(':')?;

        let nanos: i64 = ts_str.parse().ok()?;
        let timestamp = DateTime::from_timestamp_nanos(nanos);
        let id: Uuid = id_str.parse().ok()?;

        Some(Cursor { timestamp, id })
    }
}
