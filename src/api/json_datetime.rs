use chrono::Utc;

pub fn get_iso8601_timestamp() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}
