use glam::Vec3;
use crate::geometry::icosphere::IcoSphere;
use crate::geometry::kdtree3d::KDTree3D;
use crate::geometry::fbm::fbm_perlin_noise;
use std::f32::consts::PI;
use wgpu::util::DeviceExt;


#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vec3Padded {
    x: f32,
    y: f32,
    z: f32,
    _pad: f32, // padding pour correspondre à vec3<f32> aligné sur 16
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Params {
    radius: f32,
    sea_level: f32,
    height_amplitude: f32,
    continent_octaves: u32,
    continent_persistence: f32,
    continent_noise_scale: f32,
    big_mountain_octaves: u32,
    big_mountain_persistence: f32,
    big_mountain_noise_scale: f32,
    mountain_octaves: u32,
    mountain_persistence: f32,
    mountain_noise_scale: f32,
    biome_octaves: u32,
    biome_persistence: f32,
    biome_noise_scale: f32,
}

unsafe impl bytemuck::Pod for Params {}
unsafe impl bytemuck::Zeroable for Params {}

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
pub struct Sphere {
    pub sphere_vertices: Vec<f32>,
    pub sphere_indices: Vec<u32>,
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
    pub lod_levels: Vec<Sphere>,
}

impl Planet {
    pub fn new() -> Self {
        Planet {
            max_subdivision: 9,
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
        }
    }

    pub fn generate(&mut self, subdivision: u8) {
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

    // Fonction helper pour calculer les vertices avec Perlin noise
    fn compute_vertices(&mut self, v: Vec3, index: usize) {
        // Calculer la position finale avec le rayon

        let continent_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.continent_octaves, self.continent_persistence, self.continent_noise_scale);
        let big_moutain_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.big_mountain_octaves, self.big_mountain_persistence, self.big_mountain_noise_scale);
        let moutain_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.mountain_octaves, self.mountain_persistence, self.mountain_noise_scale);
        let biome_noise : f32 = fbm_perlin_noise(v.x, v.y, v.z, self.biome_octaves, self.biome_persistence, self.biome_noise_scale);

        let latitude: f32 = v.y.acos() / PI;
        // let distance_to_equator: f32 = (latitude - 0.5).abs();
        let continent_factor: f32 = (moutain_noise * big_moutain_noise * 0.6) + (continent_noise * 0.4);
        let weight_continent: f32 = smoothstep(0.0, 0.1, continent_noise);
        let weight_big_mountain: f32 = smoothstep(0.0, 0.2, big_moutain_noise);

        let mut deformed_radius: f32 = self.radius + (continent_factor * self.height_amplitude);
        deformed_radius += weight_big_mountain * weight_continent * big_moutain_noise * self.height_amplitude / 4.0;

        let under_water: bool = deformed_radius <= self.level_sea;
        if under_water {
            deformed_radius = self.level_sea;
        }

        self.lod_max_vertices[index] = deformed_radius * v;

        if under_water {
            // équivalent du commentaire "Equator handler"
            // _lod_max_colors[i] = if on_equator {
            //     Vec3::new(1.0, 0.0, 0.0)
            // } else {
            //     get_color_from_noise(continent_factor, &ocean_palette)
            // };

            self.lod_max_colors[index] = get_color_from_noise(continent_factor, &self.biome_palettes[0]);
            return;
        }

        let altitude_normalized: f32 = (deformed_radius - self.radius) / self.height_amplitude;
        let temperature: f32 = compute_temperature(latitude, altitude_normalized, v);
        let humidity: f32 = compute_humidity(v);

        let biome_idx = get_biome_index(temperature, humidity, deformed_radius, self.level_sea);
        let biome_color: Vec3 = get_color_from_noise(biome_noise, &self.biome_palettes[biome_idx]);
        let factor: f32 = moutain_noise * big_moutain_noise;
        let mountain_color: Vec3 = get_color_from_noise(factor, &self.biome_palettes[4]);
        let abs_factor = (20.0 * factor).tanh().abs();
        let inv_mix = 0.5 - abs_factor / 2.0;
        let mix     = 0.5 + abs_factor / 2.0;

        let final_color: Vec3 = Vec3::new(
            biome_color.x * inv_mix + mountain_color.x * mix,
            biome_color.y * inv_mix + mountain_color.y * mix,
            biome_color.z * inv_mix + mountain_color.z * mix
        );

        self.lod_max_colors[index] = final_color;
    }

    // Méthodes publiques pour accéder aux données de rendu
    pub fn get_vertices(&self, lod_level: usize) -> &[f32] {
        &self.lod_levels[lod_level].sphere_vertices
    }

    pub fn get_indices(&self, lod_level: usize) -> &[u32] {
        &self.lod_levels[lod_level].sphere_indices
    }

    pub fn get_vertex_count(&self, lod_level: usize) -> usize {
        self.lod_levels[lod_level].sphere_vertices.len() / 9
    }

    pub fn get_index_count(&self, lod_level: usize) -> usize {
        self.lod_levels[lod_level].sphere_indices.len()
    }
}