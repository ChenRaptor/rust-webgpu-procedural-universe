// pub enum CelestialBody {
//     Planet(Planet),
//     Star(Star),
//     Asteroid(Asteroid),
//     // etc.
// }

// pub struct CelestialBodyHandle {
//     pub body: Rc<RefCell<CelestialBody>>,
//     is_ready: Rc<RefCell<bool>>,
//     pending: Rc<RefCell<Option<(Vec<Vertex>, Vec<u32>)>>>,
//     pub vertex_buffer: Option<wgpu::Buffer>,
//     pub index_buffer: Option<wgpu::Buffer>,
//     pub instance_buffer: Option<wgpu::Buffer>,
//     pub num_indices: u32,
//     pub instance: CelestialInstance,
//     pub is_visible: bool,
//     pub id: u32,
// }

// pub struct CelestialInstance {
//     pub position: glam::Vec3,
//     pub rotation: glam::Quat,
// }