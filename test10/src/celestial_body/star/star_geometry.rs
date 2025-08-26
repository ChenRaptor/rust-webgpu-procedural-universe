use crate::celestial_body::worker::worker_new;
use crate::celestial_body::star::star_vertex::Vertex;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_COL;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_IND;
use crate::celestial_body::LOD_SHARED_ARRAY_BUFFER_POS;
use crate::geometry::{icosphere::IcoSphere};
use std::rc::Rc;
use std::cell::RefCell;

use js_sys::{Array, Float32Array, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{MessageEvent};
use js_sys::{SharedArrayBuffer, Uint32Array, Reflect, Object};
use wasm_bindgen::JsValue;

#[derive(Clone)]
pub struct StarVertex {
    pub position: Vec<f32>,
    pub color: Vec<f32>,
    pub indice: Vec<u32>
}

impl StarVertex {
    pub fn new() -> Self {
        Self {
            position: Vec::new(),
            color: Vec::new(),
            indice:Vec::new()
        }
    }
}

pub struct StarGeometry {
    // max_subdivision: u8,
    // radius: f32,
    // sphere_vertices: Vec<f32>,
    // sphere_indices: Vec<u32>,
    pub lod_content: Vec<StarVertex>
}

impl StarGeometry {
    pub fn new() -> Self {
        Self {
            // max_subdivision: 4,
            // radius: 1.0,
            // sphere_vertices: Vec::new(),
            // sphere_indices: Vec::new(),
            lod_content: Vec::new()
        }
    }

    pub fn generate_worker(
        planet_rc: &Rc<RefCell<StarGeometry>>,
        pending: Rc<RefCell<Option<(Vec<Vertex>, Vec<u32>)>>>,
        lod: usize
    ) {
        console_error_panic_hook::set_once();

        let lod_pos = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_POS[lod]);
        let lod_col = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_COL[lod]);
        let lod_ind = SharedArrayBuffer::new(LOD_SHARED_ARRAY_BUFFER_IND[lod]);

        let config: SharedArrayBuffer = SharedArrayBuffer::new(1);
        let config_data: Uint8Array = Uint8Array::new(&config);
        config_data.set_index(0, lod as u8);
        config_data.set_index(1, 1 as u8);

        // Create worker
        let worker = worker_new("worker-geometry");

        // Create common object buffer
        let obj = Object::new();
        Reflect::set(&obj, &JsValue::from_str("lod_pos"), &lod_pos).unwrap();
        Reflect::set(&obj, &JsValue::from_str("lod_col"), &lod_col).unwrap();
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

                    let lod_ind = Reflect::get(&data, &JsValue::from_str("lod_ind")).unwrap();
                    let lod_ind = Uint32Array::new(&lod_ind);

                    let mut planet_x = planet_clone.borrow_mut();

                    planet_x.lod_content.resize(lod + 1, StarVertex::new());
                    
                    let mut vec1 = vec![0.0; lod_pos.length() as usize];
                    lod_pos.copy_to(&mut vec1[..]);
                    planet_x.lod_content[lod].position = vec1;
                    
                    let mut vec2 = vec![0.0; lod_col.length() as usize];
                    lod_col.copy_to(&mut vec2[..]);
                    planet_x.lod_content[lod].color = vec2;
                    
                    let mut vec4 = vec![0; lod_ind.length() as usize];
                    lod_ind.copy_to(&mut vec4[..]);
                    planet_x.lod_content[lod].indice = vec4;
                    
                    // planet_clone.borrow_mut().lod_ready = true;
                    let pv = &planet_x.lod_content[lod];
                    
                    let vertices = Vertex::planet_vertex_to_vertex(pv);
                    let indices = planet_x.get_indices(lod).to_vec();
                    
                    *pending_clone.borrow_mut() = Some((vertices, indices));
                    worker_clone.terminate();
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    }

    pub fn generate(&mut self, subdivision: u8) {
        let mut solid = IcoSphere::new();
        solid.generate(subdivision);

        let vertex_count = solid.vertices.len();
        let indice_count = solid.indices.len();
        let vertices = &solid.vertices;
        
        let mut star_vertex = StarVertex::new();
        let position = &mut star_vertex.position;
        let color = &mut star_vertex.color;
        let indice = &mut star_vertex.indice;
        
        position.resize(3 * vertex_count, 0.0);
        color.resize(3 * vertex_count, 0.0);
        indice.reserve(indice_count);

        // Remplir les vertices
        for (i, vertex) in vertices.iter().enumerate() {

            // Position
            position[3 * i] = vertex.x;
            position[3 * i + 1] = vertex.y;
            position[3 * i + 2] = vertex.z;

            // Couleur
            color[3 * i] = 0.7;
            color[3 * i + 1] = 0.3;
            color[3 * i + 2] = 0.3;
        }

        // Indices
        indice.extend_from_slice(&solid.indices);

        self.lod_content.resize(subdivision as usize + 1, StarVertex::new());
        self.lod_content[subdivision as usize] = star_vertex;

    }

    pub fn get_indices(&self, lod_level: usize) -> &[u32] {
        &self.lod_content[lod_level].indice
    }
}