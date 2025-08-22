use js_sys::{Array, Uint32Array, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use wasm_bindgen::JsValue;

fn main() {
    // Affiche erreur de rust dans la console JS
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"worker starting".into());

    let scope = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()));

    let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {
        web_sys::console::log_1(&"got message".into());

        let data = msg.data();
        // Vérifie si on a reçu un SharedArrayBuffer
        if let Ok(sab) = Reflect::get(&data, &JsValue::from_str("sab")) {
            if !sab.is_undefined() {
                let arr = Uint32Array::new(&sab);
                let value = arr.get_index(0);
                web_sys::console::log_1(&format!("[worker] Shared value[0] = {}", value).into());
                // Modifie la valeur pour test
                arr.set_index(1, value + 1);
                return;
            }
        }

    }) as Box<dyn Fn(MessageEvent)>);

    //  Cette ligne enregistre le closure Rust comme callback pour l’événement onmessage du worker
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    // Cette ligne indique à Rust de « lâcher » la gestion mémoire du closure, pour qu’il ne soit pas libéré à la fin de la fonction
    onmessage.forget();

    // The worker must send a message to indicate that it's ready to receive messages.
    scope
        .post_message(&Array::new().into())
        .expect("posting ready message succeeds");
}
