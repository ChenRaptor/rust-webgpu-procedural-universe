use crate::celestial_body::StarVertex;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3]
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ],
        }
    }

    // fn from_planet_buffer(buffer: &[f32], vertex_index: usize) -> Self {
    //     let start = vertex_index * 9;
    //     Self {
    //         position: [buffer[start], buffer[start + 1], buffer[start + 2]],
    //         color: [buffer[start + 3], buffer[start + 4], buffer[start + 5]],
    //         normal: [buffer[start + 6], buffer[start + 7], buffer[start + 8]],
    //     }
    // }

    pub fn planet_vertex_to_vertex(pv: &StarVertex) -> Vec<Vertex> {
        let len: usize = pv.position.len() / 3;
        let mut vertices = Vec::with_capacity(len);
        for i in 0..len {
            let position = [
                pv.position[3 * i],
                pv.position[3 * i + 1],
                pv.position[3 * i + 2],
            ];
            let color = [
                pv.color[3 * i],
                pv.color[3 * i + 1],
                pv.color[3 * i + 2],
            ];
            vertices.push(Vertex { position, color });
        }
        vertices
    }

}