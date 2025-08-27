use js_sys::{Array, Float32Array, Uint8Array};
use web_sys::{window, Blob, BlobPropertyBag, Url, Worker, MessageEvent};
use crate::celestial_body::star::star_vertex;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_COL;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_IND;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_NOR;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_POS;
use crate::celestial_body::geometry_loader::{CelestialBodyGeometry, CelestialVertex};
use crate::celestial_body::planet::planet_vertex;
use crate::celestial_body::planet::planet_geometry::PlanetVertex;
use crate::celestial_body::star::star_geometry::StarVertex;

use wasm_bindgen::{prelude::*, JsCast};
use js_sys::{SharedArrayBuffer, Uint32Array, Reflect, Object};
use wasm_bindgen::JsValue;
use std::rc::Rc;
use std::cell::RefCell;

pub fn worker_new(name: &str) -> Worker {
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

pub fn generate_worker(
    planet_rc: &Rc<RefCell<CelestialBodyGeometry>>,
    pending: Rc<RefCell<Option<(Vec<CelestialVertex>, Vec<u32>)>>>,
    lod: usize
) {
    console_error_panic_hook::set_once();

    let lod_pos = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_POS[lod]);
    let lod_col = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_COL[lod]);
    let lod_nor = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_NOR[lod]);
    let lod_ind = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_IND[lod]);

    let config: SharedArrayBuffer = SharedArrayBuffer::new(2);
    let config_data: Uint8Array = Uint8Array::new(&config);
    config_data.set_index(0, lod as u8);

    let type_id = match *planet_rc.borrow() {
        CelestialBodyGeometry::Planet(_) => 0u8,
        CelestialBodyGeometry::Star(_) => 1u8,
    };
    config_data.set_index(1, type_id);

    // Create worker
    let worker = worker_new("worker-geometry");

    // Create common object buffer
    let obj = Object::new();
    Reflect::set(&obj, &JsValue::from_str("lod_pos"), &lod_pos).unwrap();
    Reflect::set(&obj, &JsValue::from_str("lod_col"), &lod_col).unwrap();
    Reflect::set(&obj, &JsValue::from_str("lod_nor"), &lod_nor).unwrap();
    Reflect::set(&obj, &JsValue::from_str("lod_ind"), &lod_ind).unwrap();
    Reflect::set(&obj, &JsValue::from_str("config"), &config).unwrap();

    let worker_is_ready = Rc::new(RefCell::new(false));
    let worker_is_ready_clone = worker_is_ready.clone();
    let planet_clone = planet_rc.clone();
    let pending_clone = pending.clone();
    let worker_clone = worker.clone();

    let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {

        let data = msg.data();
        if !*worker_is_ready_clone.borrow() {
            if Array::is_array(&data) && Array::from(&data).length() == 0 {
                worker_clone.post_message(&obj).expect("send SharedArrayBuffer");
                *worker_is_ready_clone.borrow_mut() = true;
                return;
            }
        }

        if data.is_object() && !Array::is_array(&data) {
            if Reflect::has(&data, &JsValue::from_str("lod_pos")).unwrap_or(false) {

                let lod_pos = Reflect::get(&data, &JsValue::from_str("lod_pos")).unwrap();
                let lod_pos = Float32Array::new(&lod_pos);

                let lod_col = Reflect::get(&data, &JsValue::from_str("lod_col")).unwrap();
                let lod_col = Float32Array::new(&lod_col);

                let lod_nor = Reflect::get(&data, &JsValue::from_str("lod_nor")).unwrap();
                let lod_nor = Float32Array::new(&lod_nor);

                let lod_ind = Reflect::get(&data, &JsValue::from_str("lod_ind")).unwrap();
                let lod_ind = Uint32Array::new(&lod_ind);

                
                
                let mut planet_x = planet_clone.borrow_mut();
                
                match &mut *planet_x {
                    CelestialBodyGeometry::Planet(planet) => {
                        planet.lod_levels.resize(lod + 1, PlanetVertex::new());
                        
                        let mut vec1 = vec![0.0; lod_pos.length() as usize];
                        lod_pos.copy_to(&mut vec1[..]);
                        planet.lod_levels[lod].position = vec1;
                        
                        let mut vec2 = vec![0.0; lod_col.length() as usize];
                        lod_col.copy_to(&mut vec2[..]);
                        planet.lod_levels[lod].color = vec2;
                        
                        let mut vec3 = vec![0.0; lod_nor.length() as usize];
                        lod_nor.copy_to(&mut vec3[..]);
                        planet.lod_levels[lod].normal = vec3;
                        
                        let mut vec4 = vec![0; lod_ind.length() as usize];
                        lod_ind.copy_to(&mut vec4[..]);
                        planet.lod_levels[lod].indice = vec4;
                        
                        // planet_clone.borrow_mut().lod_ready = true;
                        let pv = &planet.lod_levels[lod];
                        
                        let vertices = planet_vertex::Vertex::planet_vertex_to_vertex(pv);
                        let vertices: Vec<CelestialVertex> = vertices.into_iter().map(CelestialVertex::Planet).collect();
                        let indices = planet.get_indices(lod).to_vec();
                        
                        *pending_clone.borrow_mut() = Some((vertices, indices));
                        worker_clone.terminate();
                    }
                    CelestialBodyGeometry::Star(star) => {
                        star.lod_content.resize(lod + 1, StarVertex::new());
                        
                        let mut vec1 = vec![0.0; lod_pos.length() as usize];
                        lod_pos.copy_to(&mut vec1[..]);
                        star.lod_content[lod].position = vec1;
                        
                        let mut vec2 = vec![0.0; lod_col.length() as usize];
                        lod_col.copy_to(&mut vec2[..]);
                        star.lod_content[lod].color = vec2;
                        
                        let mut vec4 = vec![0; lod_ind.length() as usize];
                        lod_ind.copy_to(&mut vec4[..]);
                        star.lod_content[lod].indice = vec4;
                        
                        // planet_clone.borrow_mut().lod_ready = true;
                        let pv = &star.lod_content[lod];
                        
                        let vertices = star_vertex::Vertex::planet_vertex_to_vertex(pv);
                        let vertices: Vec<CelestialVertex> = vertices.into_iter().map(CelestialVertex::Star).collect();
                        let indices = star.get_indices(lod).to_vec();
                        
                        *pending_clone.borrow_mut() = Some((vertices, indices));
                        worker_clone.terminate();
                    }
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
}