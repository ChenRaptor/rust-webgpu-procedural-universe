use crate::celestial_body::geometry_loader::CelestialBodyHandle;
use crate::camera::Plane;
use wgpu::RenderPipeline;

pub struct Manager {
    pub planet_instances: Vec<CelestialBodyHandle>,
    pub buffer_loader: Vec<u32>,
    planes: [Plane; 6],
    pub in_computing: bool,
    pub id_in_computing: u32
}

impl Manager {

    pub fn new(planets: Vec<CelestialBodyHandle>) -> Self {
        Manager {
            planet_instances: planets,
            buffer_loader: Vec::new(),
            planes: [Plane::default(); 6],
            in_computing: false,
            id_in_computing: 0
        }
    }

    pub fn set_planes(&mut self, planes: [Plane; 6])
    {
        self.planes = planes;
    }

    pub fn check_visibility_cluster(&mut self, device: &wgpu::Device)
    {
        for planet_instance in &mut self.planet_instances {
            let mut visible = true;
            for plane in &self.planes {
                if plane.normal.dot(planet_instance.instance.get_position()) + plane.d < -1.5 {
                    visible = false;
                    break;
                }
            }
            planet_instance.is_visible = visible;
            if visible && !planet_instance.is_ready()
            {
                self.buffer_loader.push(planet_instance.id)
            }
        }
        if !self.in_computing
        {
            if let Some(id) = self.buffer_loader.get(0) {
                self.id_in_computing = *id;
                if let Some(planet_handle) = self.planet_instances.iter_mut().find(|p| p.id == *id) {
                    self.in_computing = true;
                    planet_handle.generate_async(5);
                }
            }
        }
        else
        {
            if let Some(planet_handle) = self.planet_instances.iter_mut().find(|p| p.id == self.id_in_computing) {
                if planet_handle.upload_if_ready(&device)
                {
                    self.in_computing = false;
                }
                self.buffer_loader.clear();
            }
        }
    }

    pub fn render_visible_object(
        &mut self, render_pass: &mut wgpu::RenderPass,
        pipeline_render: &Vec<RenderPipeline>,
        camera_bind_group: &wgpu::BindGroup,
        time_bgl: &wgpu::BindGroup
    
    )
    {
        for planet_instance in &mut self.planet_instances {
            if planet_instance.is_visible && planet_instance.is_ready()
            {
                if let (Some(vb), Some(ib), Some(jo)) = (&planet_instance.vertex_buffer, &planet_instance.index_buffer, &planet_instance.instance_buffer) {
                    if planet_instance.get_type() == 1
                    {
                        // log::info!("STAR");
                        render_pass.set_pipeline(&pipeline_render[1]);
                    }
                    else
                    {
                        render_pass.set_pipeline(&pipeline_render[0]);
                    }
                    
                    render_pass.set_bind_group(0, camera_bind_group, &[]);
                    if planet_instance.get_type() == 1
                    {
                        render_pass.set_bind_group(1, time_bgl, &[]);
                    }
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_vertex_buffer(1, jo.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..planet_instance.num_indices, 0, 0..1);
                }
            }
        }
    }
}