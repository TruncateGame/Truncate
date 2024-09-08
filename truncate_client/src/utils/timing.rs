use time::Duration;

pub fn get_qs_tick(current_time: Duration) -> u64 {
    current_time.whole_seconds() as u64 * 4 + current_time.subsec_milliseconds() as u64 / 250
}
