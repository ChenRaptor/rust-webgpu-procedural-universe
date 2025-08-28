use crate::camera::{Camera, CameraUniform};
use crate::camera::controller::CameraController;
use wgpu::util::DeviceExt;
use wgpu::Buffer;
use wgpu::BindGroup;
use wgpu::BindGroupLayout;

pub fn init_camera_scene(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> (Camera, CameraController, CameraUniform, Buffer, BindGroupLayout, BindGroup) {
    log::info!("config.width={}, config.height={}", config.width, config.height);
    let aspect_ratio = config.width as f32 / config.height as f32;
    log::info!("aspect_ratio={}", aspect_ratio);
    let camera = Camera::new(aspect_ratio);
    let camera_controller = CameraController::new(0.2);

    let mut camera_uniform = CameraUniform::new(aspect_ratio);
    camera_uniform.update_view_proj(&camera);

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });

    (camera, camera_controller, camera_uniform, camera_buffer, camera_bind_group_layout, camera_bind_group)
}