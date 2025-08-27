use crate::celestial_body::star::star_geometry::StarGeometry;
use crate::celestial_body::PlanetGeometry;
use crate::celestial_body::planet::planet_vertex;
use crate::celestial_body::planet::planet_instance;
use crate::celestial_body::star::star_vertex;
use crate::celestial_body::star::star_instance;
use crate::celestial_body::worker::generate_worker;
use std::rc::Rc;
use std::cell::RefCell;
use glam::{Vec3, Quat};
use wgpu::util::DeviceExt;
pub enum CelestialBodyGeometry {
    Planet(PlanetGeometry),
    Star(StarGeometry)
}

impl CelestialBodyGeometry {
    pub fn get_type(&self) -> u8 {
        match self {
            CelestialBodyGeometry::Planet(_) => 0,
            CelestialBodyGeometry::Star(_) => 1,
        }
    }
}

pub enum CelestialVertex {
    Planet(planet_vertex::Vertex),
    Star(star_vertex::Vertex),
}

pub enum CelestialInstance {
    Planet(planet_instance::PlanetInstance),
    Star(star_instance::StarInstance),
}

impl CelestialInstance {
    pub fn get_position(&self) -> Vec3 {
        match self {
            CelestialInstance::Planet(p) => p.position,
            CelestialInstance::Star(s) => s.position,
        }
    }
}

pub struct CelestialBodyHandle {
    pub body: Rc<RefCell<CelestialBodyGeometry>>,
    is_ready: Rc<RefCell<bool>>,
    pending: Rc<RefCell<Option<(Vec<CelestialVertex>, Vec<u32>)>>>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub instance_buffer: Option<wgpu::Buffer>,
    pub num_indices: u32,
    pub instance: CelestialInstance,
    pub is_visible: bool,
    pub id: u32
}

impl CelestialBodyHandle {
    pub fn new(body: CelestialBodyGeometry,position: Vec3, rotation: Quat, id: u32) -> Self {
        let instance  = match &body {
            CelestialBodyGeometry::Planet(_) => CelestialInstance::Planet(planet_instance::PlanetInstance { position, rotation }),
            CelestialBodyGeometry::Star(_) => CelestialInstance::Star(star_instance::StarInstance { position, rotation }),
        };
        Self {
            body: Rc::new(RefCell::new(body)),
            is_ready: Rc::new(RefCell::new(false)),
            pending: Rc::new(RefCell::new(None)),
            vertex_buffer: None,
            index_buffer: None,
            instance_buffer: None,
            num_indices: 0,
            instance,
            is_visible: false,
            id
        }
    }

    pub fn generate_async(&self, lod: usize) {
        let body = self.body.clone();
        let pending_flag = self.pending.clone();

        generate_worker(&body, pending_flag, lod);
    }

    pub fn upload_if_ready(&mut self, device: &wgpu::Device) -> bool {

        if let Some((vertices, indices)) = self.pending.borrow_mut().take() {
            
            match vertices.get(0) {
                Some(CelestialVertex::Planet(_)) => {
                    let verts: Vec<_> = vertices.into_iter().filter_map(|v| {
                        if let CelestialVertex::Planet(p) = v { Some(p) } else { None }
                    }).collect();
                    self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: bytemuck::cast_slice(&verts),
                        usage: wgpu::BufferUsages::VERTEX,
                    }));
                }
                Some(CelestialVertex::Star(_)) => {
                    let verts: Vec<_> = vertices.into_iter().filter_map(|v| {
                        if let CelestialVertex::Star(s) = v { Some(s) } else { None }
                    }).collect();
                    self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: bytemuck::cast_slice(&verts),
                        usage: wgpu::BufferUsages::VERTEX,
                    }));
                }
                None => {}
            }

            self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }));

            match &self.instance {
                CelestialInstance::Planet(p) => {
                    let instance_data = vec![planet_instance::PlanetInstance::to_raw(p)];
                    self.instance_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX,
                    }));
                }
                CelestialInstance::Star(s) => {
                    let instance_data = vec![star_instance::StarInstance::to_raw(s)];
                    self.instance_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX,
                    }));
                }
            }

            self.num_indices = indices.len() as u32;
            log::info!("Planet is uploaded");

            *self.is_ready.borrow_mut() = true;
            return true;
        }
        return false;
    }

    pub fn is_ready(&self) -> bool {
        *self.is_ready.borrow()
    }

    pub fn get_type(&self) -> u8 {
        self.body.borrow().get_type()
    }

}
