use js_sys::{Array, Uint32Array, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use wasm_bindgen::JsValue;

const P: [usize; 512] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225,
    140, 36, 103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148,
    247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32,
    57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122,
    60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54,
    65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169,
    200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64,
    52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212,
    207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213,
    119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9,
    129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104,
    218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162, 241,
    81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157,
    184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93,
    222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225,
    140, 36, 103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148,
    247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32,
    57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122,
    60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54,
    65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169,
    200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64,
    52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212,
    207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213,
    119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9,
    129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104,
    218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162, 241,
    81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157,
    184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93,
    222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
];

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
fn grad(hash: usize, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { 
        y 
    } else if h == 12 || h == 14 { 
        x 
    } else { 
        z 
    };
    
    let u_sign = if (h & 1) != 0 { -u } else { u };
    let v_sign = if (h & 2) != 0 { -v } else { v };
    
    u_sign + v_sign
}

/// Fonction de bruit de Perlin 3D
pub fn perlin_noise_3d(x: f32, y: f32, z: f32) -> f32 {
    ensure_initialized();
    
    let x_int = (x.floor() as i32 & 255) as usize;
    let y_int = (y.floor() as i32 & 255) as usize;
    let z_int = (z.floor() as i32 & 255) as usize;
    
    let x_frac = x - x.floor();
    let y_frac = y - y.floor();
    let z_frac = z - z.floor();
    
    let u = fade(x_frac);
    let v = fade(y_frac);
    let w = fade(z_frac);
    
    unsafe {
        let a = P[x_int] + y_int;
        let aa = P[a] + z_int;
        let ab = P[a + 1] + z_int;
        let b = P[x_int + 1] + y_int;
        let ba = P[b] + z_int;
        let bb = P[b + 1] + z_int;
        
        let res = lerp(w,
            lerp(v,
                lerp(u, grad(P[aa], x_frac, y_frac, z_frac), 
                        grad(P[ba], x_frac - 1.0, y_frac, z_frac)),
                lerp(u, grad(P[ab], x_frac, y_frac - 1.0, z_frac), 
                        grad(P[bb], x_frac - 1.0, y_frac - 1.0, z_frac))
            ),
            lerp(v,
                lerp(u, grad(P[aa + 1], x_frac, y_frac, z_frac - 1.0), 
                        grad(P[ba + 1], x_frac - 1.0, y_frac, z_frac - 1.0)),
                lerp(u, grad(P[ab + 1], x_frac, y_frac - 1.0, z_frac - 1.0), 
                        grad(P[bb + 1], x_frac - 1.0, y_frac - 1.0, z_frac - 1.0))
            )
        );
        
        res
    }
}

/// Fonction de bruit fractal (FBM) utilisant le bruit de Perlin 3D.
/// 
/// # Arguments
/// * `x`, `y`, `z` - Coordonnées dans l'espace 3D
/// * `octaves` - Nombre d'octaves pour le bruit fractal
/// * `persistence` - Facteur de persistance pour l'amplitude des octaves
/// * `scale` - Facteur d'échelle pour la fréquence du bruit
/// 
/// # Returns
/// Valeur de bruit normalisée entre -1 et 1
pub fn fbm_perlin_noise(x: f32, y: f32, z: f32, octaves: u8, persistence: f32, scale: f32) -> f32 {
    let mut total = 0.0;
    let mut frequency = scale;
    let mut amplitude = 1.0;
    let mut max_value = 0.0;
    
    for _ in 0..octaves {
        total += perlin_noise_3d(x * frequency, y * frequency, z * frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }
    
    total / max_value  // normalisation approximative entre -1 et 1
}

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



pub async fn generate(&mut self, subdivision: u8) {
    // Équivalent de static std::unique_ptr<KDTree3D> kdTreeMax;
    static mut KD_TREE_MAX: Option<KDTree3D> = None;

    if subdivision > self.max_subdivision {
        println!("Planet: Invalid subdivision {}, max is {}", subdivision, self.max_subdivision);
        return;
    }

    if self.lod_levels.len() <= subdivision as usize {
        println!("Planet: Resizing lod_levels from {} to {}", self.lod_levels.len(), subdivision + 1);
        self.lod_levels.resize(self.max_subdivision as usize + 1, Sphere {
            sphere_vertices: Vec::new(),
            sphere_indices: Vec::new(),
        });
    }

    if !self.lod_levels[subdivision as usize].sphere_vertices.is_empty() {
        return;
    }

    // Générer la subdivision maximale si nécessaire
    if self.lod_max_solid.is_none() {
        println!("Planet: Generating max subdivision solid for LOD {}", self.max_subdivision);
        let mut max_solid = IcoSphere::new();
        max_solid.generate(self.max_subdivision as u32);

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
    self.lod_levels[subdivision as usize].sphere_vertices = self.sphere_vertices.clone();
    self.lod_levels[subdivision as usize].sphere_indices = self.sphere_indices.clone();
}