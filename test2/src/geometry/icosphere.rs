use glam::Vec3;
use std::collections::HashMap;

pub struct IcoSphere {
	pub vertices: Vec<Vec3>,
	pub indices: Vec<u32>,
	pub subdivisions: u32,
	middle_point_cache: HashMap<u64, u32>,
}

impl IcoSphere {
	pub fn new() -> Self {
		IcoSphere {
			vertices: Vec::new(),
			indices: Vec::new(),
			subdivisions: 0,
			middle_point_cache: HashMap::new(),
		}
	}

	fn get_middle_point(&mut self, p1: u32, p2: u32) -> u32 {
		let (min, max) = if p1 < p2 { (p1, p2) } else { (p2, p1) };
		let key = ((min as u64) << 32) | (max as u64);
		if let Some(&idx) = self.middle_point_cache.get(&key) {
			return idx;
		}
        let middle = (self.vertices[p1 as usize] + self.vertices[p2 as usize]) * 0.5;
        let normalized = middle.normalize();
        self.vertices.push(normalized);
        let idx = (self.vertices.len() - 1) as u32;
        self.middle_point_cache.insert(key, idx);
        idx
    }

	fn init_icosahedron(&mut self) {
		self.vertices.clear();
		self.indices.clear();
	self.middle_point_cache.clear();

	let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
		let mut v = [
			Vec3::new(-1.0,  t,   0.0),
			Vec3::new( 1.0,  t,   0.0),
			Vec3::new(-1.0, -t,   0.0),
			Vec3::new( 1.0, -t,   0.0),
			Vec3::new( 0.0, -1.0,  t  ),
			Vec3::new( 0.0,  1.0,  t  ),
			Vec3::new( 0.0, -1.0, -t  ),
			Vec3::new( 0.0,  1.0, -t  ),
			Vec3::new(  t,   0.0, -1.0),
			Vec3::new(  t,   0.0,  1.0),
			Vec3::new(-t,   0.0, -1.0),
			Vec3::new(-t,   0.0,  1.0),
		];
		for vert in v.iter_mut() {
			*vert = vert.normalize();
			self.vertices.push(*vert);
		}
		let idx: [u32; 60] = [
			0,11,5, 0,5,1, 0,1,7, 0,7,10, 0,10,11,
			1,5,9, 5,11,4, 11,10,2, 10,7,6, 7,1,8,
			3,9,4, 3,4,2, 3,2,6, 3,6,8, 3,8,9,
			4,9,5, 2,4,11, 6,2,10, 8,6,7, 9,8,1
		];
		self.indices.extend_from_slice(&idx);
	}

	fn subdivide_triangles(&mut self) {
		let indices = self.indices.clone();
		let mut new_indices = Vec::with_capacity(indices.len() * 4);
		for tri in indices.chunks(3) {
			let v1 = tri[0];
			let v2 = tri[1];
			let v3 = tri[2];
			let a = self.get_middle_point(v1, v2);
			let b = self.get_middle_point(v2, v3);
			let c = self.get_middle_point(v3, v1);

			new_indices.extend_from_slice(&[v1, a, c]);
			new_indices.extend_from_slice(&[v2, b, a]);
			new_indices.extend_from_slice(&[v3, c, b]);
			new_indices.extend_from_slice(&[a, b, c]);
		}
		self.indices = new_indices;
	}

	pub fn generate(&mut self, subdivisions: u32) {
		self.init_icosahedron();
		for _ in 0..subdivisions {
			self.subdivide_triangles();
		}
		self.subdivisions = subdivisions;
	}
}
