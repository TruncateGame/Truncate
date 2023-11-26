self.onmessage = function (e) {
    if (e.data.action === 'loadWasm' && e.data.wasm_bytes) {
        console.log("[WORKER] Loading backend");
        // Initialize wasm_bindgen (synchronous)
        const backend = e.data.backend;
        importScripts(backend);

        console.log("[WORKER] Loading Truncate");
        wasm_bindgen(e.data.wasm_bytes)
            .then(on_wasm_loaded)
            .catch(on_wasm_error);

        function on_wasm_loaded() {
            console.log("[WORKER] Successfully loaded Truncate");
        }

        function on_wasm_error(error) {
            console.log(`[WORKER] Failed to load Truncate: ${error.toString()}`);
        }
    } else if (e.data.action === 'backchannel') {
        const result = wasm_bindgen.backchannel(e.data.msg);
        self.postMessage({ action: 'result', result, id: e.data.id });
    }
};