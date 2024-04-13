macro_rules! tr_log {
    ($log:block) => {{
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&$log.into());
        println!("{:?}", $log);
    }};
}

macro_rules! current_time {
    () => {{
        // We have to go through the instant crate as
        // most std time functions are not implemented
        // in Rust's wasm targets.
        // instant::SystemTime::now() conditionally uses
        // a js function on wasm targets, and otherwise aliases
        // to the std SystemTime type.
        instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("Please don't play Truncate earlier than 1970")
    }};
}
pub(crate) use current_time;
