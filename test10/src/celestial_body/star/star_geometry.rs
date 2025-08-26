use glam::Vec3;

pub struct StarVertex {
    pub position: Vec<f32>,
    pub color: Vec<f32>,
    pub indice: Vec<u32>
}

impl StarVertex {
    pub fn new() -> Self {
        PlanetVertex {
            position: Vec::new(),
            color: Vec::new(),
            indice:Vec::new()
        }
    }
}

pub struct Star {
    max_subdivision: u8,
    radius: f32,
    sphere_vertices: Vec<f32>,
    sphere_indices: Vec<u32>,
    lod: Vec<StarVertex>
}

impl Star {
    pub fn new() -> Self {
        Planet {
            max_subdivision: 4,
            radius: 1.0,
            sphere_vertices: Vec::new(),
            sphere_indices: Vec::new(),
            lod: Vec::new()
        }
    }

    pub fn generate(&mut self, subdivision: u8) {
        let mut solid = IcoSphere::new();
        solid.generate(subdivision);

        let vertex_count = solid.vertices.len();
        let indice_count = solid.indices.len();
        let vertices = &solid.vertices;
        
        let mut star_vertex = StarVertex::new();
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

        self.lod.resize(subdivision as usize + 1, StarVertex::new());
        self.lod[subdivision as usize] = star_vertex;

    }
}