use js_sys::{Array, Float32Array, Reflect, Uint32Array, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use wasm_bindgen::JsValue;
use webworker_example::celestial_body::planet::planet_geometry::PlanetGeometry;
use webworker_example::celestial_body::star::star_geometry::StarGeometry;
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
        if let Ok(config) = Reflect::get(&data, &JsValue::from_str("config")) {
            if !config.is_undefined() {

                let config_data = Uint8Array::new(&config);
                let lod = config_data.get_index(0);
                let body_type = config_data.get_index(1);

                if body_type == 0 {
                    // Planète : avec normal
                    web_sys::console::log_1(&"PLANET".into());
                    let mut planet = PlanetGeometry::new();
                    planet.generate(lod);
                    let planet_vertex = &planet.lod_levels[lod as usize];

                    // Normal
                    let lod9_normal = Reflect::get(&data, &JsValue::from_str("lod_nor")).unwrap_or(JsValue::NULL);
                    let lod9_normal_arr = Float32Array::new(&lod9_normal);
                    lod9_normal_arr.copy_from(&planet_vertex.normal[..]);

                    // Position
                    let lod9_position = Reflect::get(&data, &JsValue::from_str("lod_pos")).unwrap_or(JsValue::NULL);
                    let lod9_position_arr = Float32Array::new(&lod9_position);
                    lod9_position_arr.copy_from(&planet_vertex.position[..]);

                    // Color
                    let lod9_color = Reflect::get(&data, &JsValue::from_str("lod_col")).unwrap_or(JsValue::NULL);
                    let lod9_color_arr = Float32Array::new(&lod9_color);
                    lod9_color_arr.copy_from(&planet_vertex.color[..]);

                    // Indice
                    let lod9_indice = Reflect::get(&data, &JsValue::from_str("lod_ind")).unwrap_or(JsValue::NULL);
                    let lod9_indice_arr = Uint32Array::new(&lod9_indice);
                    lod9_indice_arr.copy_from(&planet_vertex.indice[..]);

                    scope_clone.post_message(&data).expect("Worker send response");
                    return;
                } else if body_type == 1 {
                    // Étoile : pas de normal
                    let mut star = StarGeometry::new();
                    
                    web_sys::console::log_1(&"STAR".into());
                    star.generate(lod);
                    let star_vertex = &star.lod_content[lod as usize];


                    // Position
                    let lod9_position = Reflect::get(&data, &JsValue::from_str("lod_pos")).unwrap_or(JsValue::NULL);
                    let lod9_position_arr = Float32Array::new(&lod9_position);
                    lod9_position_arr.copy_from(&star_vertex.position[..]);

                    // Color
                    let lod9_color = Reflect::get(&data, &JsValue::from_str("lod_col")).unwrap_or(JsValue::NULL);
                    let lod9_color_arr = Float32Array::new(&lod9_color);
                    lod9_color_arr.copy_from(&star_vertex.color[..]);

                    // Indice
                    let lod9_indice = Reflect::get(&data, &JsValue::from_str("lod_ind")).unwrap_or(JsValue::NULL);
                    let lod9_indice_arr = Uint32Array::new(&lod9_indice);
                    lod9_indice_arr.copy_from(&star_vertex.indice[..]);

                    scope_clone.post_message(&data).expect("Worker send response");
                    return;
                }
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