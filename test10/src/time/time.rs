use wgpu::util::DeviceExt;
use wgpu::Buffer;
use wgpu::BindGroup;
use wgpu::BindGroupLayout;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    pub time: f32
}

impl TimeUniform {
    pub fn new(time: f32) -> Self
    {
        Self {
            time,
        }
    }
}

pub struct TimeUniformGroup {
    pub time_uniform: TimeUniform,
    pub time_buffer: Buffer,
    pub time_bgl: BindGroupLayout,
    pub time_bg: BindGroup
}

impl TimeUniformGroup {
    pub fn new(time_uniform: TimeUniform, time_buffer: Buffer, time_bgl: BindGroupLayout, time_bg: BindGroup) -> Self {
        Self {
            time_uniform,
            time_buffer,
            time_bgl,
            time_bg
        }
    }
}

pub fn init_time_scene(device: &wgpu::Device) -> TimeUniformGroup {
    let time_uniform = TimeUniform::new(0.0);

    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[time_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let time_bgl =
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

    let time_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &time_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: time_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });

    TimeUniformGroup::new(time_uniform, time_buffer, time_bgl, time_bg)
}