macro_rules! tr_log {
    ($log:block) => {{
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&$log.into());
        println!("{:?}", $log);
    }};
}
