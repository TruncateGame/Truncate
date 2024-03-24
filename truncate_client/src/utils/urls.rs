pub fn back_to_menu() {
    #[cfg(target_arch = "wasm32")]
    {
        let host = web_sys::window().unwrap().location().host().unwrap();
        let protocol = web_sys::window().unwrap().location().protocol().unwrap();

        _ = web_sys::window()
            .unwrap()
            .location()
            .replace(&format!("{protocol}//{host}/"));
    }
}
