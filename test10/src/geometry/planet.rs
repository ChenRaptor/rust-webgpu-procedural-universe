use glam::Vec3;
use crate::geometry::{icosphere::IcoSphere};
use crate::geometry::kdtree3d::KDTree3D;
use crate::geometry::fbm::fbm_perlin_noise;
use crate::Vertex;
use std::f32::consts::PI;
use std::rc::Rc;
use std::cell::RefCell;
use wgpu::util::DeviceExt;

use js_sys::{Array, Float32Array};
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


#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vec3Padded {
    x: f32,
    y: f32,
    z: f32,
    _pad: f32, // padding pour correspondre à vec3<f32> aligné sur 16
}

// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// struct Params {
//     radius: f32,
//     sea_level: f32,
//     height_amplitude: f32,
//     continent_octaves: u32,
//     continent_persistence: f32,
//     continent_noise_scale: f32,
//     big_mountain_octaves: u32,
//     big_mountain_persistence: f32,
//     big_mountain_noise_scale: f32,
//     mountain_octaves: u32,
//     mountain_persistence: f32,
//     mountain_noise_scale: f32,
//     biome_octaves: u32,
//     biome_persistence: f32,
//     biome_noise_scale: f32,
// }

// unsafe impl bytemuck::Pod for Params {}
// unsafe impl bytemuck::Zeroable for Params {}

struct ColorPoint {
    color: Vec3,
    key: f32,
}

impl ColorPoint {
    pub fn new(color: Vec3, key: f32) -> Self {
        Self { color, key }
    }
    
    pub fn from_hex(hex: u32, key: f32) -> Self {
        let color = Vec3::new(
            ((hex >> 16) & 0xFF) as f32 / 255.0,
            ((hex >> 8) & 0xFF) as f32 / 255.0,
            (hex & 0xFF) as f32 / 255.0,
        );
        Self::new(color, key)
    }
}

fn get_biome_index(temperature: f32, humidity: f32, altitude: f32, sea_level: f32) -> usize {
    if altitude < sea_level {
        // Biome::Ocean
        0
    } else if temperature > 0.7 {
        if humidity < 0.3 {
            // Biome::Desert
            1
        } else {
            // Biome::Forest
            2
        }
    } else if temperature > 0.3 {
        if humidity < 0.3 {
            // Biome::Tundra // steppe simplifiée
            3
        } else {
            // Biome::Forest
            2
        }
    } else {
        if humidity < 0.3 {
            // Biome::Tundra
            3
        } else {
            // Biome::Snow
            5
        }
    }
}

fn get_color_from_noise(noise_value: f32, palette: &[ColorPoint]) -> Vec3 {
    if palette.is_empty() {
        return Vec3::new(0.0, 0.0, 0.0);
    }

    if noise_value <= palette.first().unwrap().key {
        return palette.first().unwrap().color;
    }
    if noise_value >= palette.last().unwrap().key {
        return palette.last().unwrap().color;
    }

    for i in 0..palette.len() - 1 {
        if noise_value >= palette[i].key && noise_value <= palette[i + 1].key {
            let t = (noise_value - palette[i].key) / (palette[i + 1].key - palette[i].key);
            return palette[i].color.lerp(palette[i + 1].color, t);
        }
    }

    palette.last().unwrap().color
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn compute_temperature(latitude: f32, altitude: f32, v: Vec3) -> f32 {
    // Température de base selon latitude et altitude
    let mut base_temp = 1.0 - (latitude - 0.5).abs() * 2.0 - altitude * 0.7;
    base_temp = base_temp.clamp(0.0, 1.0);

    // Ajout de bruit FBM multi-échelle
    let temp_noise1 = fbm_perlin_noise(v.x, v.y, v.z, 4, 0.9, 2.0);
    let temp_noise2 = fbm_perlin_noise(v.x, v.y, v.z, 4, 0.9, 20.0);
    let temperature = base_temp + 0.3 * temp_noise1 + 0.15 * temp_noise2;

    temperature // Valeur de température calculée
}

fn compute_humidity(v: Vec3) -> f32 {
    // Humidité basée sur le bruit FBM
    let humidity_noise1 = fbm_perlin_noise(v.x + 100.0, v.y + 100.0, v.z + 100.0, 4, 0.5, 2.0);
    let humidity_noise2 = fbm_perlin_noise(v.x + 200.0, v.y + 200.0, v.z + 200.0, 4, 0.6, 20.0);
    let mut humidity = 0.7 * humidity_noise1 + 0.3 * humidity_noise2;
    humidity = (humidity + 1.0) * 0.5 * 0.70;

    humidity // Valeur d'humidité calculée
}

#[derive(Clone)]
pub struct PlanetVertex {
    pub position: Vec<f32>,
    pub color: Vec<f32>,
    pub normal: Vec<f32>,
    pub indice: Vec<u32>
}

#[derive(Clone)]
pub struct Sphere {
    pub sphere_vertices: Vec<f32>,
    pub sphere_indices: Vec<u32>,
}


impl PlanetVertex {
    pub fn new() -> Self {
        PlanetVertex {
            position: Vec::new(),
            color: Vec::new(),
            normal:Vec::new(),
            indice:Vec::new()
        }
    }
}

pub struct Planet {
    max_subdivision: u8,
    radius: f32,
    level_sea: f32,
    height_amplitude: f32,
    continent_octaves: u8,
    continent_persistence: f32,
    continent_noise_scale: f32,
    big_mountain_octaves: u8,
    big_mountain_persistence: f32,
    big_mountain_noise_scale: f32,
    mountain_octaves: u8,
    mountain_persistence: f32,
    mountain_noise_scale: f32,
    biome_octaves: u8,
    biome_persistence: f32,
    biome_noise_scale: f32,
    biome_palettes: [Vec<ColorPoint>; 6],
    sphere_vertices: Vec<f32>,
    sphere_indices: Vec<u32>,
    lod_max_solid: Option<IcoSphere>,
    lod_max_vertices: Vec<Vec3>,
    lod_max_colors: Vec<Vec3>,
    kd_tree_max: Option<KDTree3D>,
    pub lod_levels: Vec<PlanetVertex>,
    pub lod_ready: bool,
    pub lod_levels2: Vec<Sphere>,
    pub vertex: Vec<Vertex>
}

impl Planet {
    pub fn new() -> Self {
        Planet {
            max_subdivision: 5,
            radius: 1.0,
            level_sea: 0.998,
            height_amplitude: 0.05,
            continent_octaves: 3,
            continent_persistence: 0.5,
            continent_noise_scale: 0.8,
            big_mountain_octaves: 8,
            big_mountain_persistence: 0.7,
            big_mountain_noise_scale: 4.0,
            mountain_octaves: 8,
            mountain_persistence: 0.9,
            mountain_noise_scale: 2.0,
            biome_octaves: 3,
            biome_persistence: 0.6,
            biome_noise_scale: 5.0,
            biome_palettes: [
                vec![
                    ColorPoint::from_hex(0x000030, -0.2),
                    ColorPoint::from_hex(0x000041, -0.1),
                    ColorPoint::from_hex(0x35698C, -0.005),
                    ColorPoint::from_hex(0x40E0D0, 0.0)
                ],
                vec![
                    ColorPoint::from_hex(0xC2B280, 0.0),
                    ColorPoint::from_hex(0xEEDC82, 0.5),
                    ColorPoint::from_hex(0xFFE4B5, 1.0),
                ],
                vec![
                    ColorPoint::from_hex(0x05400A, -1.0),
                    ColorPoint::from_hex(0x527048, 0.0),
                    ColorPoint::from_hex(0x7CFC00, 1.0),
                ],
                vec![
                    ColorPoint::from_hex(0x9FA8A3, 0.0),
                    ColorPoint::from_hex(0xDCE3E1, 1.0),
                ],
                vec![
                    ColorPoint::from_hex(0x000000, 0.0),
                    ColorPoint::from_hex(0x222222, 0.01),
                    ColorPoint::from_hex(0x333333, 0.05),
                    ColorPoint::from_hex(0x666666, 0.09),
                    ColorPoint::from_hex(0x777777, 0.1),
                    ColorPoint::from_hex(0x8c8c9c, 0.9),
                ],
                vec![
                    ColorPoint::from_hex(0xEEEEEE, 0.0),
                    ColorPoint::from_hex(0xFFFFFF, 1.0),
                ]],
            sphere_vertices: Vec::new(),
            sphere_indices: Vec::new(),
            lod_max_solid: None,
            lod_max_vertices: Vec::new(),
            lod_max_colors: Vec::new(),
            kd_tree_max: None,
            lod_levels: Vec::new(),
            lod_ready: false,
            lod_levels2: Vec::new(),
            vertex: Vec::new()
        }
    }

    // /// Version GPU multithread de la génération utilisant un compute shader
    // pub async fn generate_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, subdivision: u8) {
    //     static mut KD_TREE_MAX: Option<KDTree3D> = None;

    //     if subdivision > self.max_subdivision {
    //         println!("Planet: Invalid subdivision {}, max is {}", subdivision, self.max_subdivision);
    //         return;
    //     }

    //     if self.lod_levels.len() <= subdivision as usize {
    //         self.lod_levels.resize(self.max_subdivision as usize + 1, Sphere {
    //             sphere_vertices: Vec::new(),
    //             sphere_indices: Vec::new(),
    //         });
    //     }

    //     if !self.lod_levels[subdivision as usize].sphere_vertices.is_empty() {
    //         return;
    //     }

    //     // Générer la subdivision maximale si nécessaire
    //     if self.lod_max_solid.is_none() {
    //         println!("Planet: Generating max subdivision solid for LOD {}", self.max_subdivision);
    //         let mut max_solid = IcoSphere::new();
    //         max_solid.generate(self.max_subdivision as u32);

    //         // Construire le k-d tree sur les sommets de subdivision max
    //         let mut points_max = Vec::new();
    //         points_max.reserve(max_solid.vertices.len());
    //         for vertex in &max_solid.vertices {
    //             points_max.push(*vertex);
    //         }

    //         unsafe {
    //             KD_TREE_MAX = Some(KDTree3D::new(&points_max));
    //         }

    //         // Préparer les buffers pour le compute shader
    //         self.lod_max_vertices.resize(points_max.len(), Vec3::ZERO);
    //         self.lod_max_colors.resize(points_max.len(), Vec3::ZERO);

    //         // === COMPUTE SHADER GPU ===
    //         self.compute_vertices_gpu(device, queue, &max_solid.vertices).await;

    //         self.lod_max_solid = Some(max_solid);
    //     }

    //     // Le reste de la logique reste identique...
    //     let solid = if subdivision == self.max_subdivision {
    //         self.lod_max_solid.as_ref().unwrap()
    //     } else {
    //         self.lod_max_solid.as_ref().unwrap()
    //     };

    //     let vertex_count = solid.vertices.len();
    //     let index_count = solid.indices.len();
    //     let vertices = &solid.vertices;

    //     self.sphere_vertices.clear();
    //     self.sphere_indices.clear();
    //     self.sphere_vertices.resize(vertex_count * 9, 0.0);
    //     self.sphere_indices.reserve(index_count);

    //     // Remplir les vertices (utilise les résultats GPU)
    //     for (i, vertex) in vertices.iter().enumerate() {
    //         let nearest_index = unsafe {
    //             if let Some(ref kdtree) = KD_TREE_MAX {
    //                 kdtree.nearest_neighbor(*vertex)
    //             } else {
    //                 0
    //             }
    //         };
    //         let nearest_vertex = self.lod_max_vertices[nearest_index];
    //         let nearest_color = self.lod_max_colors[nearest_index];

    //         // Position
    //         self.sphere_vertices[9 * i + 0] = nearest_vertex.x;
    //         self.sphere_vertices[9 * i + 1] = nearest_vertex.y;
    //         self.sphere_vertices[9 * i + 2] = nearest_vertex.z;

    //         // Couleur
    //         self.sphere_vertices[9 * i + 3] = nearest_color.x;
    //         self.sphere_vertices[9 * i + 4] = nearest_color.y;
    //         self.sphere_vertices[9 * i + 5] = nearest_color.z;
    //     }

    //     // Indices et normales (identique à la version CPU)
    //     self.sphere_indices.extend_from_slice(&solid.indices);

    //     let mut normals = vec![Vec3::ZERO; vertex_count];
    //     for triangle in self.sphere_indices.chunks(3) {
    //         let i0 = triangle[0] as usize;
    //         let i1 = triangle[1] as usize;
    //         let i2 = triangle[2] as usize;

    //         let v0 = Vec3::new(
    //             self.sphere_vertices[9 * i0],
    //             self.sphere_vertices[9 * i0 + 1],
    //             self.sphere_vertices[9 * i0 + 2],
    //         );
    //         let v1 = Vec3::new(
    //             self.sphere_vertices[9 * i1],
    //             self.sphere_vertices[9 * i1 + 1],
    //             self.sphere_vertices[9 * i1 + 2],
    //         );
    //         let v2 = Vec3::new(
    //             self.sphere_vertices[9 * i2],
    //             self.sphere_vertices[9 * i2 + 1],
    //             self.sphere_vertices[9 * i2 + 2],
    //         );

    //         let edge1 = v1 - v0;
    //         let edge2 = v2 - v0;
    //         let normal = edge1.cross(edge2).normalize();

    //         normals[i0] += normal;
    //         normals[i1] += normal;
    //         normals[i2] += normal;
    //     }

    //     for (i, normal) in normals.iter().enumerate() {
    //         let n = normal.normalize();
    //         self.sphere_vertices[9 * i + 6] = n.x;
    //         self.sphere_vertices[9 * i + 7] = n.y;
    //         self.sphere_vertices[9 * i + 8] = n.z;
    //     }

    //     self.lod_levels[subdivision as usize].sphere_vertices = self.sphere_vertices.clone();
    //     self.lod_levels[subdivision as usize].sphere_indices = self.sphere_indices.clone();
    // }

    // /// Fonction GPU qui remplace la boucle CPU par un compute shader
    // async fn compute_vertices_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vec3]) {
    //     let vertex_count = vertices.len();

    //     // 1. Convertir les vertices en format compatible GPU
    //     let input_vertices: Vec<Vec3Padded> = vertices.iter()
    //         .map(|v| Vec3Padded { x: v.x, y: v.y, z: v.z, _pad: 0.0 })
    //         .collect();

    //     // 2. Créer les buffers d'entrée
    //     let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //         label: Some("Input Vertices Buffer"),
    //         contents: bytemuck::cast_slice(&input_vertices),
    //         usage: wgpu::BufferUsages::STORAGE,
    //     });

    //     // 3. Créer les buffers de sortie
    //     let output_positions_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    //         label: Some("Output Positions Buffer"),
    //         size: (vertex_count * std::mem::size_of::<Vec3Padded>()) as u64,
    //         usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    //         mapped_at_creation: false,
    //     });

    //     let output_colors_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    //         label: Some("Output Colors Buffer"),
    //         size: (vertex_count * std::mem::size_of::<Vec3Padded>()) as u64,
    //         usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    //         mapped_at_creation: false,
    //     });

    //     // 4. Créer le buffer de paramètres
    //     let params = Params {
    //         radius: self.radius,
    //         sea_level: self.level_sea,
    //         height_amplitude: self.height_amplitude,
    //         continent_octaves: self.continent_octaves as u32,
    //         continent_persistence: self.continent_persistence,
    //         continent_noise_scale: self.continent_noise_scale,
    //         big_mountain_octaves: self.big_mountain_octaves as u32,
    //         big_mountain_persistence: self.big_mountain_persistence,
    //         big_mountain_noise_scale: self.big_mountain_noise_scale,
    //         mountain_octaves: self.mountain_octaves as u32,
    //         mountain_persistence: self.mountain_persistence,
    //         mountain_noise_scale: self.mountain_noise_scale,
    //         biome_octaves: self.biome_octaves as u32,
    //         biome_persistence: self.biome_persistence,
    //         biome_noise_scale: self.biome_noise_scale,
    //     };

    //     let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //         label: Some("Params Buffer"),
    //         contents: bytemuck::bytes_of(&params),
    //         usage: wgpu::BufferUsages::UNIFORM,
    //     });

    //     // 5. Charger le compute shader
    //     let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    //         label: Some("Planet Compute Shader"),
    //         source: wgpu::ShaderSource::Wgsl(include_str!("compute_vertices.wgsl").into()),
    //     });

    //     // 6. Créer le compute pipeline
    //     let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    //         label: Some("Planet Compute Pipeline"),
    //         layout: None,
    //         module: &shader,
    //         entry_point: Some("main"),
    //         compilation_options: wgpu::PipelineCompilationOptions::default(),
    //         cache: None,
    //     });

    //     // 7. Créer le bind group
    //     let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    //     let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    //         label: Some("Planet Compute Bind Group"),
    //         layout: &bind_group_layout,
    //         entries: &[
    //             wgpu::BindGroupEntry {
    //                 binding: 0,
    //                 resource: params_buffer.as_entire_binding(),
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 1,
    //                 resource: input_buffer.as_entire_binding(),
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 2,
    //                 resource: output_positions_buffer.as_entire_binding(),
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 3,
    //                 resource: output_colors_buffer.as_entire_binding(),
    //             },
    //         ],
    //     });

    //     // 8. Créer les buffers de lecture pour rapatrier les données
    //     let readback_positions = device.create_buffer(&wgpu::BufferDescriptor {
    //         label: Some("Readback Positions"),
    //         size: (vertex_count * std::mem::size_of::<Vec3Padded>()) as u64,
    //         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    //         mapped_at_creation: false,
    //     });

    //     let readback_colors = device.create_buffer(&wgpu::BufferDescriptor {
    //         label: Some("Readback Colors"),
    //         size: (vertex_count * std::mem::size_of::<Vec3Padded>()) as u64,
    //         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    //         mapped_at_creation: false,
    //     });

    //     // 9. Exécuter le compute shader
    //     let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //         label: Some("Planet Compute Encoder"),
    //     });

    //     {
    //         let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    //             label: Some("Planet Compute Pass"),
    //             timestamp_writes: None,
    //         });
    //         cpass.set_pipeline(&compute_pipeline);
    //         cpass.set_bind_group(0, &bind_group, &[]);
    //         // Dispatch avec des workgroups de 64 threads
    //         cpass.dispatch_workgroups((vertex_count as u32 + 63) / 64, 1, 1);
    //     }

    //     // 10. Copier les résultats vers les buffers de lecture
    //     encoder.copy_buffer_to_buffer(
    //         &output_positions_buffer, 0,
    //         &readback_positions, 0,
    //         (vertex_count * std::mem::size_of::<Vec3Padded>()) as u64,
    //     );

    //     encoder.copy_buffer_to_buffer(
    //         &output_colors_buffer, 0,
    //         &readback_colors, 0,
    //         (vertex_count * std::mem::size_of::<Vec3Padded>()) as u64,
    //     );

    //     // 11. Soumettre
    //     queue.submit(Some(encoder.finish()));

    //     // 12. Lire les résultats avec mapping asynchrone
    //     let positions_slice = readback_positions.slice(..);
    //     let colors_slice = readback_colors.slice(..);

    //     // Utiliser des channels pour attendre que les mappings soient terminés
    //     let (positions_sender, positions_receiver) = std::sync::mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
    //     let (colors_sender, colors_receiver) = std::sync::mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();

    //     // Lancer les mappings asynchrones
    //     positions_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         let _ = positions_sender.send(result);
    //     });
    //     colors_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         let _ = colors_sender.send(result);
    //     });

    //     // Attendre que les deux mappings soient terminés
    //     let _positions_result = positions_receiver.recv().unwrap().unwrap();
    //     let _colors_result = colors_receiver.recv().unwrap().unwrap();

    //     // Maintenant on peut lire les données en toute sécurité
    //     let positions_data = positions_slice.get_mapped_range();
    //     let colors_data = colors_slice.get_mapped_range();

    //     let gpu_positions: &[Vec3Padded] = bytemuck::cast_slice(&positions_data);
    //     let gpu_colors: &[Vec3Padded] = bytemuck::cast_slice(&colors_data);

    //     // 13. Copier les résultats dans les structures de la planète
    //     for (i, (pos, col)) in gpu_positions.iter().zip(gpu_colors.iter()).enumerate() {
    //         self.lod_max_vertices[i] = Vec3::new(pos.x, pos.y, pos.z);
    //         self.lod_max_colors[i] = Vec3::new(col.x, col.y, col.z);
    //     }

    //     drop(positions_data);
    //     drop(colors_data);
    //     readback_positions.unmap();
    //     readback_colors.unmap();

    //     println!("GPU compute completed for {} vertices", vertex_count);
    // }

    pub fn generate_worker(
        planet_rc: &Rc<RefCell<Planet>>,
        pending: Rc<RefCell<Option<(Vec<Vertex>, Vec<u32>)>>>
        // subdivision: u8
    ) {
        console_error_panic_hook::set_once();

        // // 7864326 * 4 = 31457304
        // let lod9_position = SharedArrayBuffer::new(31457304);
        // let lod9_position: Float32Array = Float32Array::new(&lod9_position);

        // // 7864326 * 4 = 31457304
        // let lod9_color = SharedArrayBuffer::new(31457304);
        // let lod9_color: Float32Array = Float32Array::new(&lod9_color);

        // // 7864326 * 4 = 31457304
        // let lod9_normal = SharedArrayBuffer::new(31457304);
        // let lod9_normal: Float32Array = Float32Array::new(&lod9_normal);

        // // 15728640 * 4 = 62914560
        // let lod9_indice = SharedArrayBuffer::new(62914560);
        // let lod9_indice: Uint32Array = Uint32Array::new(&lod9_indice);


        // 30726 * 4 = 122904
        let lod9_position = SharedArrayBuffer::new(122904);
        // let lod9_position_data: Float32Array = Float32Array::new(&lod9_position);

        // 30726 * 4 = 122904
        let lod9_color = SharedArrayBuffer::new(122904);
        // let lod9_color_data: Float32Array = Float32Array::new(&lod9_color);

        // 30726 * 4 = 122904
        let lod9_normal = SharedArrayBuffer::new(122904);
        // let lod9_normal_data: Float32Array = Float32Array::new(&lod9_normal);

        // 61440 * 4 = 245760
        let lod9_indice = SharedArrayBuffer::new(245760);
        // let lod9_indice_data: Uint32Array = Uint32Array::new(&lod9_indice);


        // Create worker
        let worker = worker_new("worker-geometry");

        // Create common object buffer
        let obj = Object::new();

        Reflect::set(&obj, &JsValue::from_str("lod9_position"), &lod9_position).unwrap();
        Reflect::set(&obj, &JsValue::from_str("lod9_color"), &lod9_color).unwrap();
        Reflect::set(&obj, &JsValue::from_str("lod9_normal"), &lod9_normal).unwrap();
        Reflect::set(&obj, &JsValue::from_str("lod9_indice"), &lod9_indice).unwrap();

        // Reflect::set(&obj, &JsValue::from_str("lod8_vertex"), &lod8_vertex).unwrap();
        // Reflect::set(&obj, &JsValue::from_str("lod8_indice"), &lod8_indice).unwrap();

        // Reflect::set(&obj, &JsValue::from_str("lod7_vertex"), &lod7_vertex).unwrap();
        // Reflect::set(&obj, &JsValue::from_str("lod7_indice"), &lod7_indice).unwrap();

        // Reflect::set(&obj, &JsValue::from_str("lod6_vertex"), &lod6_vertex).unwrap();
        // Reflect::set(&obj, &JsValue::from_str("lod6_indice"), &lod6_indice).unwrap();

        // Reflect::set(&obj, &JsValue::from_str("lod5_vertex"), &lod5_vertex).unwrap();
        // Reflect::set(&obj, &JsValue::from_str("lod5_indice"), &lod5_indice).unwrap();

        // Reflect::set(&obj, &JsValue::from_str("lod4_vertex"), &lod4_vertex).unwrap();
        // Reflect::set(&obj, &JsValue::from_str("lod4_indice"), &lod4_indice).unwrap();

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
                if Reflect::has(&data, &JsValue::from_str("lod9_position")).unwrap_or(false) {

                    let subdivision: usize = 5; // must match buffer allocation and usage

                    let lod9_position = Reflect::get(&data, &JsValue::from_str("lod9_position")).unwrap();
                    let lod9_position = Float32Array::new(&lod9_position);

                    let lod9_color = Reflect::get(&data, &JsValue::from_str("lod9_color")).unwrap();
                    let lod9_color = Float32Array::new(&lod9_color);

                    let lod9_normal = Reflect::get(&data, &JsValue::from_str("lod9_normal")).unwrap();
                    let lod9_normal = Float32Array::new(&lod9_normal);

                    let lod9_indice = Reflect::get(&data, &JsValue::from_str("lod9_indice")).unwrap();
                    let lod9_indice = Uint32Array::new(&lod9_indice);


                    let mut planet_x = planet_clone.borrow_mut();

                    planet_x.lod_levels.resize(subdivision + 1, PlanetVertex::new());
                    
                    let mut vec1 = vec![0.0; lod9_position.length() as usize];
                    lod9_position.copy_to(&mut vec1[..]);
                    planet_x.lod_levels[subdivision].position = vec1;
                    
                    let mut vec2 = vec![0.0; lod9_color.length() as usize];
                    lod9_color.copy_to(&mut vec2[..]);
                    planet_x.lod_levels[subdivision].color = vec2;
                    
                    let mut vec3 = vec![0.0; lod9_normal.length() as usize];
                    lod9_normal.copy_to(&mut vec3[..]);
                    planet_x.lod_levels[subdivision].normal = vec3;
                    
                    let mut vec4 = vec![0; lod9_indice.length() as usize];
                    lod9_indice.copy_to(&mut vec4[..]);
                    planet_x.lod_levels[subdivision].indice = vec4;
                    
                    // planet_clone.borrow_mut().lod_ready = true;
                    let pv = &planet_x.lod_levels[subdivision];
                    
                    let vertices = Vertex::planet_vertex_to_vertex(pv);
                    let vertices2 = Vertex::planet_vertex_to_vertex(pv);
                    planet_x.vertex = vertices;
                    let indices = planet_x.get_indices(subdivision).to_vec();
                    
                    *pending_clone.borrow_mut() = Some((vertices2, indices));
    
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
        
        let mut planet_vertex = PlanetVertex::new();
        let position = &mut planet_vertex.position;
        let color = &mut planet_vertex.color;
        let vnormal = &mut planet_vertex.normal;
        let indice = &mut planet_vertex.indice;
        
        position.resize(3 * vertex_count, 0.0);
        color.resize(3 * vertex_count, 0.0);
        vnormal.resize(3 * vertex_count, 0.0);
        indice.reserve(indice_count);

        // Remplir les vertices
        for (i, vertex) in vertices.iter().enumerate() {

            let (v, c) = self.compute_vertex_data(*vertex);

            // Position
            position[3 * i] = v.x;
            position[3 * i + 1] = v.y;
            position[3 * i + 2] = v.z;

            // Couleur
            color[3 * i] = c.x;
            color[3 * i + 1] = c.y;
            color[3 * i + 2] = c.z;
        }

        // Indices
        indice.extend_from_slice(&solid.indices);

        // Calcul des normales par accumulation
        let mut normals = vec![Vec3::ZERO; vertex_count];
        for triangle in indice.chunks(3) {
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;

            let v0 = Vec3::new(
                position[3 * i0],
                position[3 * i0 + 1],
                position[3 * i0 + 2],
            );
            let v1 = Vec3::new(
                position[3 * i1],
                position[3 * i1 + 1],
                position[3 * i1 + 2],
            );
            let v2 = Vec3::new(
                position[3 * i2],
                position[3 * i2 + 1],
                position[3 * i2 + 2],
            );

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(edge2).normalize();

            normals[i0] += normal;
            normals[i1] += normal;
            normals[i2] += normal;
        }

        // Normaliser et assigner les normales
        for (i, normal) in normals.iter().enumerate() {
            let n = normal.normalize();
            vnormal[3 * i] = n.x;
            vnormal[3 * i + 1] = n.y;
            vnormal[3 * i + 2] = n.z;
        }

        // Sauvegarder dans le niveau LOD
        // self.lod_levels[subdivision as usize].sphere_vertices = self.sphere_vertices.clone();
        // self.lod_levels[subdivision as usize].sphere_indices = self.sphere_indices.clone();

        self.lod_levels.resize(subdivision as usize + 1, PlanetVertex::new());
        self.lod_levels[subdivision as usize] = planet_vertex;

    }

    pub fn generate2(&mut self, subdivision: u8) {
        // Équivalent de static std::unique_ptr<KDTree3D> kdTreeMax;
        static mut KD_TREE_MAX: Option<KDTree3D> = None;

        if subdivision > self.max_subdivision {
            println!("Planet: Invalid subdivision {}, max is {}", subdivision, self.max_subdivision);
            return;
        }

        if self.lod_levels.len() <= subdivision as usize {
            println!("Planet: Resizing lod_levels from {} to {}", self.lod_levels.len(), subdivision + 1);
            self.lod_levels2.resize(self.max_subdivision as usize + 1, Sphere {
                sphere_vertices: Vec::new(),
                sphere_indices: Vec::new(),
            });
        }

        if !self.lod_levels2[subdivision as usize].sphere_vertices.is_empty() {
            return;
        }

        // Générer la subdivision maximale si nécessaire
        if self.lod_max_solid.is_none() {
            println!("Planet: Generating max subdivision solid for LOD {}", self.max_subdivision);
            let mut max_solid = IcoSphere::new();
            max_solid.generate(self.max_subdivision as u8);

            // Construire le k-d tree sur les sommets de subdivision max
            let mut points_max = Vec::new();
            points_max.reserve(max_solid.vertices.len());
            for vertex in &max_solid.vertices {
                points_max.push(*vertex);
            }

            unsafe {
                KD_TREE_MAX = Some(KDTree3D::new(&points_max));
            }

            // Calculer valeurs pour subdivision max
            self.lod_max_vertices.resize(points_max.len(), Vec3::ZERO);
            self.lod_max_colors.resize(points_max.len(), Vec3::ZERO);

            for (i, vertex) in max_solid.vertices.iter().enumerate() {
                let (v, c) = self.compute_vertex_data(*vertex);
                self.lod_max_vertices[i] = v;
                self.lod_max_colors[i] = c;
            }

            self.lod_max_solid = Some(max_solid);
        }

        // Choisir la source de géométrie
        let solid = if subdivision == self.max_subdivision {
            println!("Planet: Using precomputed max subdivision solid for LOD {}", self.max_subdivision);
            self.lod_max_solid.as_ref().unwrap()
        } else {
            // Pour les subdivisions inférieures, on devrait créer une nouvelle IcoSphere
            // mais gardons l'existante pour l'instant
            self.lod_max_solid.as_ref().unwrap()
        };

        let vertex_count = solid.vertices.len();
        let index_count = solid.indices.len();
        let vertices = &solid.vertices;

        self.sphere_vertices.clear();
        self.sphere_indices.clear();
        self.sphere_vertices.resize(vertex_count * 9, 0.0);
        self.sphere_indices.reserve(index_count);

        // Remplir les vertices
        for (i, vertex) in vertices.iter().enumerate() {
            // Trouver le vertex le plus proche dans la subdivision max avec KDTree
            let nearest_index = unsafe {
                if let Some(ref kdtree) = KD_TREE_MAX {
                    kdtree.nearest_neighbor(*vertex)
                } else {
                    0 // Fallback si pas de KDTree
                }
            };
            let nearest_vertex = self.lod_max_vertices[nearest_index];
            let nearest_color = self.lod_max_colors[nearest_index];

            // Position
            self.sphere_vertices[9 * i + 0] = nearest_vertex.x;
            self.sphere_vertices[9 * i + 1] = nearest_vertex.y;
            self.sphere_vertices[9 * i + 2] = nearest_vertex.z;

            // Couleur
            self.sphere_vertices[9 * i + 3] = nearest_color.x;
            self.sphere_vertices[9 * i + 4] = nearest_color.y;
            self.sphere_vertices[9 * i + 5] = nearest_color.z;
        }

        // Indices
        self.sphere_indices.extend_from_slice(&solid.indices);

        // Calcul des normales par accumulation
        let mut normals = vec![Vec3::ZERO; vertex_count];
        for triangle in self.sphere_indices.chunks(3) {
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;

            let v0 = Vec3::new(
                self.sphere_vertices[9 * i0],
                self.sphere_vertices[9 * i0 + 1],
                self.sphere_vertices[9 * i0 + 2],
            );
            let v1 = Vec3::new(
                self.sphere_vertices[9 * i1],
                self.sphere_vertices[9 * i1 + 1],
                self.sphere_vertices[9 * i1 + 2],
            );
            let v2 = Vec3::new(
                self.sphere_vertices[9 * i2],
                self.sphere_vertices[9 * i2 + 1],
                self.sphere_vertices[9 * i2 + 2],
            );

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(edge2).normalize();

            normals[i0] += normal;
            normals[i1] += normal;
            normals[i2] += normal;
        }

        // Normaliser et assigner les normales
        for (i, normal) in normals.iter().enumerate() {
            let n = normal.normalize();
            self.sphere_vertices[9 * i + 6] = n.x;
            self.sphere_vertices[9 * i + 7] = n.y;
            self.sphere_vertices[9 * i + 8] = n.z;
        }

        // Sauvegarder dans le niveau LOD
        self.lod_levels2[subdivision as usize].sphere_vertices = self.sphere_vertices.clone();
        self.lod_levels2[subdivision as usize].sphere_indices = self.sphere_indices.clone();
    }

    // Fonction helper pour calculer les vertices avec Perlin noise (thread-safe)
    fn compute_vertex_data(&self, v: Vec3) -> (Vec3, Vec3) {
        // Calculer la position finale avec le rayon
        let continent_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.continent_octaves, self.continent_persistence, self.continent_noise_scale);
        let big_moutain_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.big_mountain_octaves, self.big_mountain_persistence, self.big_mountain_noise_scale);
        let moutain_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.mountain_octaves, self.mountain_persistence, self.mountain_noise_scale);
        let biome_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.biome_octaves, self.biome_persistence, self.biome_noise_scale);

        let latitude: f32 = v.y.acos() / PI;
        let continent_factor: f32 = (moutain_noise * big_moutain_noise * 0.6) + (continent_noise * 0.4);
        let weight_continent: f32 = smoothstep(0.0, 0.1, continent_noise);
        let weight_big_mountain: f32 = smoothstep(0.0, 0.2, big_moutain_noise);

        let mut deformed_radius: f32 = self.radius + (continent_factor * self.height_amplitude);
        deformed_radius += weight_big_mountain * weight_continent * big_moutain_noise * self.height_amplitude / 4.0;

        let under_water: bool = deformed_radius <= self.level_sea;
        if under_water {
            deformed_radius = self.level_sea;
        }

        let final_vertex = deformed_radius * v;

        let final_color = if under_water {
            get_color_from_noise(continent_factor, &self.biome_palettes[0])
        } else {
            let altitude_normalized: f32 = (deformed_radius - self.radius) / self.height_amplitude;
            let temperature: f32 = compute_temperature(latitude, altitude_normalized, v);
            let humidity: f32 = compute_humidity(v);

            let biome_idx = get_biome_index(temperature, humidity, deformed_radius, self.level_sea);
            let biome_color: Vec3 = get_color_from_noise(biome_noise, &self.biome_palettes[biome_idx]);
            let factor: f32 = moutain_noise * big_moutain_noise;
            let mountain_color: Vec3 = get_color_from_noise(factor, &self.biome_palettes[4]);
            let abs_factor = (20.0 * factor).tanh().abs();
            let inv_mix = 0.5 - abs_factor / 2.0;
            let mix = 0.5 + abs_factor / 2.0;

            Vec3::new(
                biome_color.x * inv_mix + mountain_color.x * mix,
                biome_color.y * inv_mix + mountain_color.y * mix,
                biome_color.z * inv_mix + mountain_color.z * mix
            )
        };

        (final_vertex, final_color)
    }

    pub fn get_positions(&self, lod_level: usize) -> &[f32] {
        &self.lod_levels[lod_level].position
    }


    pub fn get_colors(&self, lod_level: usize) -> &[f32] {
        &self.lod_levels[lod_level].color
    }


    pub fn get_normals(&self, lod_level: usize) -> &[f32] {
        &self.lod_levels[lod_level].normal
    }


    pub fn get_indices(&self, lod_level: usize) -> &[u32] {
        &self.lod_levels[lod_level].indice
    }


    pub fn get_vertex_count(&self, lod_level: usize) -> usize {
        self.lod_levels[lod_level].position.len() / 3
    }

    pub fn get_index_count(&self, lod_level: usize) -> usize {
        self.lod_levels[lod_level].indice.len()
    }


    pub fn get_vertices2(&self, lod_level: usize) -> &[f32] {
        &self.lod_levels2[lod_level].sphere_vertices
    }

    pub fn get_indices2(&self, lod_level: usize) -> &[u32] {
        &self.lod_levels2[lod_level].sphere_indices
    }

    pub fn get_vertex_count2(&self, lod_level: usize) -> usize {
        self.lod_levels2[lod_level].sphere_vertices.len() / 9
    }

    pub fn get_index_count2(&self, lod_level: usize) -> usize {
        self.lod_levels2[lod_level].sphere_indices.len()
    }
}


pub struct PlanetHandle {
    pub planet: Rc<RefCell<Planet>>,
    is_ready: Rc<RefCell<bool>>,
    pending: Rc<RefCell<Option<(Vec<Vertex>, Vec<u32>)>>>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_indices: u32,
}

impl PlanetHandle {
    pub fn new(planet: Planet) -> Self {
        Self {
            planet: Rc::new(RefCell::new(planet)),
            is_ready: Rc::new(RefCell::new(false)),
            pending: Rc::new(RefCell::new(None)),
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
        }
    }

    pub fn generate_async(&self) {
        let planet = self.planet.clone();
        let pending_flag = self.pending.clone();


        Planet::generate_worker(&planet, pending_flag);
    }

    pub fn upload_if_ready(&mut self, device: &wgpu::Device) {

        if let Some((vertices, indices)) = self.pending.borrow_mut().take() {
            
            self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }));

            self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }));

            self.num_indices = indices.len() as u32;
            log::info!("Planet is uploaded");

            *self.is_ready.borrow_mut() = true;
        }
    }

    pub fn is_ready(&self) -> bool {
        *self.is_ready.borrow()
    }
}
