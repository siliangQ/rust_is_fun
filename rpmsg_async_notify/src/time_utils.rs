use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn unix_time(t: SystemTime) -> Duration {
    return t.duration_since(UNIX_EPOCH).expect("Time went backwards");
}
