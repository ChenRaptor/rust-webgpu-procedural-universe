use js_sys::{Array, Uint32Array, Float32Array, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use wasm_bindgen::JsValue;
use webworker_example::geometry::planet::Planet;

fn main() {
    // Affiche erreur de rust dans la console JS
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"worker starting".into());

    let scope = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()));

    let scope_clone = scope.clone();
    let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {
        web_sys::console::log_1(&"got message".into());

        let data: JsValue = msg.data();
        // Vérifie si on a reçu un SharedArrayBuffer
        if let Ok(lod9_position) = Reflect::get(&data, &JsValue::from_str("lod9_position")) {
            if !lod9_position.is_undefined() {

                let subdivision: usize = 5;
                
                let mut planet = Planet::new();
                planet.generate(subdivision as u8);
                let planet_vertex = &planet.lod_levels[subdivision];


                web_sys::console::log_1(&format!("[worker] planet_vertex.indice = {}", planet_vertex.indice.len()).into());
                web_sys::console::log_1(&format!("[worker] planet_vertex.position = {}", planet_vertex.position.len()).into());
                web_sys::console::log_1(&format!("[worker] planet_vertex.color = {}", planet_vertex.color.len()).into());
                web_sys::console::log_1(&format!("[worker] planet_vertex.normal = {}", planet_vertex.normal.len()).into());

                // Position
                let lod9_position_arr = Float32Array::new(&lod9_position);
                lod9_position_arr.copy_from(&planet_vertex.position[..]);


                // Color
                let lod9_color = Reflect::get(&data, &JsValue::from_str("lod9_color")).unwrap_or(JsValue::NULL);
                let lod9_color_arr = Float32Array::new(&lod9_color);
                lod9_color_arr.copy_from(&planet_vertex.color[..]);


                // Normal
                let lod9_normal = Reflect::get(&data, &JsValue::from_str("lod9_normal")).unwrap_or(JsValue::NULL);
                let lod9_normal_arr = Float32Array::new(&lod9_normal);
                lod9_normal_arr.copy_from(&planet_vertex.normal[..]);

                // Indice
                let lod9_indice = Reflect::get(&data, &JsValue::from_str("lod9_indice")).unwrap_or(JsValue::NULL);
                let lod9_indice_arr = Uint32Array::new(&lod9_indice);
                lod9_indice_arr.copy_from(&planet_vertex.indice[..]);

                web_sys::console::log_1(&format!("[worker] Vertex count = {}", planet.get_vertex_count(subdivision)).into());
                scope_clone.post_message(&data).expect("Worker send response");
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