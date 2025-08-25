
use webworker_example::run;
use js_sys::Array;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{window, Blob, BlobPropertyBag, MessageEvent, Url, Worker};
use js_sys::{SharedArrayBuffer, Uint32Array, Reflect, Object};
use wasm_bindgen::JsValue;

fn worker_new(name: &str) -> Worker {
    let origin = window()
        .expect("window to be available")
        .location()
        .origin()
        .expect("origin to be available");

    let script = Array::new();
    script.push(
        &format!(r#"importScripts("{origin}/{name}.js");wasm_bindgen("{origin}/{name}_bg.wasm");"#)
            .into(),
    );

    let blob = Blob::new_with_str_sequence_and_options(
        &script,
        BlobPropertyBag::new().type_("text/javascript"),
    )
    .expect("blob creation succeeds");

    let url = Url::create_object_url_with_blob(&blob).expect("url creation succeeds");

    Worker::new(&url).expect("failed to spawn worker")
}

fn main() {
    console_error_panic_hook::set_once();

    // let sab = SharedArrayBuffer::new(1024); // 1024 bytes
    // let arr = Uint32Array::new(&sab);
    // arr.set_index(0, 123); // Exemple d'écriture

    // let worker = worker_new("worker");

    // let obj = Object::new();
    // // Crée une propriété "sab" sur l’objet JavaScript obj et lui assigne la valeur du SharedArrayBuffer Rust sab.
    // Reflect::set(&obj, &JsValue::from_str("sab"), &sab).unwrap();

    // // On attend le message "ready" du worker avant d'envoyer le buffer
    // let mut sent = false;

    // /*
    // On crée un worker_clone parce que la variable worker doit être utilisée à l’intérieur du closure (le handler d’événement), mais Rust impose que toutes les variables capturées par un closure move soient possédées ou clonées.

    // - Le closure peut être appelé plusieurs fois, et il doit posséder sa propre référence au worker pour pouvoir appeler post_message.
    // - Worker implémente le trait Clone, ce qui permet de dupliquer la référence JS sous-jacente sans créer un nouveau worker.
    //  */
    // let worker_clone = worker.clone();
    // let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {
    //     let data = msg.data();
    //     // Si le worker est prêt (message vide)
    //     if !sent {
    //         // On vérifie que le message est un Array vide (comme dans worker.rs)
    //         if Array::is_array(&data) && Array::from(&data).length() == 0 {
    //             worker_clone.post_message(&obj).expect("send SharedArrayBuffer");
    //             sent = true;
    //             return;
    //         }
    //     }
    //     // Si on reçoit un Array avec des nombres (protocole fallback)
    //     if Array::is_array(&data) {
    //         let array = Array::from(&data);
    //         if array.length() >= 3 {
    //             let a = array.get(0).as_f64().unwrap_or(0.0) as u32;
    //             let b = array.get(1).as_f64().unwrap_or(0.0) as u32;
    //             let result = array.get(2).as_f64().unwrap_or(0.0) as u32;
    //             web_sys::console::log_1(&format!("{a} x {b} = {result} - JOJOBA").into());
    //             return;
    //         }
    //     }
    //     // Sinon, on affiche la valeur du SharedArrayBuffer
    //     let value0 = arr.get_index(0);
    //     let value1 = arr.get_index(1);
    //     web_sys::console::log_1(&format!("[main] Shared value[0] = {}, value[1] = {}", value0, value1).into());
    // }) as Box<dyn FnMut(MessageEvent)>);
    // worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    // onmessage.forget();

    run().unwrap();
}
