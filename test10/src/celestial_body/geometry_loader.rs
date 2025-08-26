// use crate::celestial_body::PlanetGeometry;

// pub enum CelestialBody {
//     Planet(PlanetGeometry),
// }

// // pub struct CelestialBodyHandle {
// //     pub body: Rc<RefCell<CelestialBody>>,
// //     is_ready: Rc<RefCell<bool>>,
// //     pending: Rc<RefCell<Option<(Vec<Vertex>, Vec<u32>)>>>,
// //     pub vertex_buffer: Option<wgpu::Buffer>,
// //     pub index_buffer: Option<wgpu::Buffer>,
// //     pub instance_buffer: Option<wgpu::Buffer>,
// //     pub num_indices: u32,
// //     pub instance: CelestialInstance,
// //     pub is_visible: bool,
// //     pub id: u32,
// // }

// // pub struct CelestialInstance {
// //     pub position: glam::Vec3,
// //     pub rotation: glam::Quat,
// // }

// pub struct CelestialBodyHandle {
//     pub body: Rc<RefCell<CelestialBody>>,
//     is_ready: Rc<RefCell<bool>>,
//     pending: Rc<RefCell<Option<(Vec<Vertex>, Vec<u32>)>>>,
//     pub vertex_buffer: Option<wgpu::Buffer>,
//     pub index_buffer: Option<wgpu::Buffer>,
//     pub instance_buffer: Option<wgpu::Buffer>,
//     pub num_indices: u32,
//     pub instance: PlanetInstance,
//     pub is_visible: bool,
//     pub id: u32

// }




// impl PlanetHandle {
//     pub fn new(planet: Planet, position: Vec3, rotation: Quat, id: u32) -> Self {
//         Self {
//             planet: Rc::new(RefCell::new(planet)),
//             is_ready: Rc::new(RefCell::new(false)),
//             pending: Rc::new(RefCell::new(None)),
//             vertex_buffer: None,
//             index_buffer: None,
//             instance_buffer: None,
//             num_indices: 0,
//             instance: PlanetInstance {position, rotation},
//             is_visible: false,
//             id
//         }
//     }

//     pub fn generate_async(&self, lod: usize) {
//         let planet = self.planet.clone();
//         let pending_flag = self.pending.clone();


//         Planet::generate_worker(&planet, pending_flag, lod);
//     }

//     pub fn upload_if_ready(&mut self, device: &wgpu::Device) -> bool {

//         if let Some((vertices, indices)) = self.pending.borrow_mut().take() {
            
//             self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                 label: Some("Vertex Buffer"),
//                 contents: bytemuck::cast_slice(&vertices),
//                 usage: wgpu::BufferUsages::VERTEX,
//             }));

//             self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                 label: Some("Index Buffer"),
//                 contents: bytemuck::cast_slice(&indices),
//                 usage: wgpu::BufferUsages::INDEX,
//             }));

//             let instance_data = vec![PlanetInstance::to_raw(&self.instance)];
//             self.instance_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                     label: Some("Instance Buffer"),
//                     contents: bytemuck::cast_slice(&instance_data),
//                     usage: wgpu::BufferUsages::VERTEX,
//             }));

//             self.num_indices = indices.len() as u32;
//             log::info!("Planet is uploaded");

//             *self.is_ready.borrow_mut() = true;
//             return true;
//         }
//         return false;
//     }

//     pub fn is_ready(&self) -> bool {
//         *self.is_ready.borrow()
//     }

// }
