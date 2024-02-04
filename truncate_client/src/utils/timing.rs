use instant::Duration;

pub fn get_qs_tick(current_time: Duration) -> u64 {
    current_time.as_secs() * 4 + current_time.subsec_millis() as u64 / 250
}
