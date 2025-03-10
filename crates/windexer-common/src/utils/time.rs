use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::NaiveDateTime;

pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn duration_since(timestamp: i64) -> Duration {
    let now = current_timestamp();
    Duration::from_secs((now - timestamp).max(0) as u64)
}

pub fn format_timestamp(timestamp: i64) -> String {
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
        .unwrap_or_default();
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

pub fn is_recent(timestamp: i64, max_age: Duration) -> bool {
    duration_since(timestamp) <= max_age
}

pub fn unix_timestamp_to_datetime(timestamp: i64) -> Option<NaiveDateTime> {
    chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
        .map(|dt| dt.naive_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timestamp_functions() {
        let now = current_timestamp();
        assert!(now > 0);
        
        let hour_ago = now - 3600;
        let duration = duration_since(hour_ago);
        assert!(duration.as_secs() >= 3600);
        
        let formatted = format_timestamp(now);
        assert!(!formatted.is_empty());
        
        assert!(is_recent(now, Duration::from_secs(10)));
        assert!(!is_recent(hour_ago, Duration::from_secs(60)));
    }
}