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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn cursor_roundtrip() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let cursor = Cursor::new(now, id);

        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();

        assert!((decoded.timestamp - now).num_microseconds().unwrap().abs() < 1000);
        assert_eq!(decoded.id, id);
    }

    #[test]
    fn cursor_decode_invalid_base64() {
        assert!(Cursor::decode("not-valid-base64!@#$").is_none());
    }

    #[test]
    fn cursor_decode_invalid_format() {
        let encoded = URL_SAFE_NO_PAD.encode(b"just_a_string");
        assert!(Cursor::decode(&encoded).is_none());
    }

    #[test]
    fn cursor_decode_invalid_uuid() {
        let encoded = URL_SAFE_NO_PAD.encode(b"12345:not-a-uuid");
        assert!(Cursor::decode(&encoded).is_none());
    }

    #[test]
    fn cursor_decode_empty() {
        assert!(Cursor::decode("").is_none());
    }
}
