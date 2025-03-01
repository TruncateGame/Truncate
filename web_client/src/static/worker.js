let loadedWasm = false;

const resolve_backchannel = (e) => {
    if (!loadedWasm) {
        return setTimeout(() => resolve_backchannel(e), 10);
    }

    const result = wasm_bindgen.backchannel(e.data.msg);
    self.postMessage({ action: 'result', result, id: e.data.id });
}

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

            loadedWasm = true;
            wasm_bindgen.backchannel_setup();
            
        }

        function on_wasm_error(error) {
            console.log(`[WORKER] Failed to load Truncate: ${error.toString()}`);
        }
    } else if (e.data.action === 'backchannel') {
        resolve_backchannel(e);
    }
};