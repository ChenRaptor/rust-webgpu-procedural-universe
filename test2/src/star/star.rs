use glam::Vec3;

pub struct Star {
    max_subdivision: u8,
    radius: f32,
    sphere_vertices: Vec<f32>,
    sphere_indices: Vec<u32>,
}

impl Star {
    pub fn new() -> Self {
        Planet {
            max_subdivision: 4,
            radius: 1.0,
            sphere_vertices: Vec::new(),
            sphere_indices: Vec::new(),
        }
    }

    pub fn generate(&mut self, subdivision: u8) {
        let mut max_solid = IcoSphere::new();
        max_solid.generate(self.max_subdivision as u32);

        let vertex_count = max_solid.vertices.len();
        let index_count = max_solid.indices.len();

        self.sphere_vertices.clear();
        self.sphere_indices.clear();
        self.sphere_vertices.resize(vertex_count * 9, 0.0);
        self.sphere_indices.reserve(index_count);

        for (i, vertex) in vertices.iter().enumerate() {
            self.sphere_vertices[9 * i + 0] = vertex.x;
            self.sphere_vertices[9 * i + 1] = vertex.y;
            self.sphere_vertices[9 * i + 2] = vertex.z;

            self.sphere_vertices[9 * i + 3] = 1.0;
            self.sphere_vertices[9 * i + 4] = 0.0;
            self.sphere_vertices[9 * i + 5] = 0.0;
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
    }
}