pub mod planet {
    pub mod planet_geometry;
    pub mod render_pipeline;
    pub mod planet_vertex;
    pub mod planet_instance;
}

pub mod star {
    pub mod star_geometry;
    pub mod star_vertex;
}

pub mod geometry_loader;
pub mod worker;

pub use planet::planet_geometry::{PlanetGeometry, PlanetHandle, PlanetVertex, PlanetInstance};
pub use planet::planet_vertex::Vertex;
pub use planet::planet_instance::InstanceRaw;
pub use planet::render_pipeline::planet_render_pipeline;

pub use star::star_geometry::{StarVertex};

pub const LOD_SHARED_ARRAY_BUFFER_POS: [u32; 10] = [144, 504, 1944, 7704, 30744, 122904, 491544, 1966104, 7864344, 31457304];
pub const LOD_SHARED_ARRAY_BUFFER_COL: [u32; 10] = [144, 504, 1944, 7704, 30744, 122904, 491544, 1966104, 7864344, 31457304];
pub const LOD_SHARED_ARRAY_BUFFER_NOR: [u32; 10] = [144, 504, 1944, 7704, 30744, 122904, 491544, 1966104, 7864344, 31457304];
pub const LOD_SHARED_ARRAY_BUFFER_IND: [u32; 10] = [240, 960, 3840, 15360, 61440, 245760, 983040, 3932160, 15728640, 62914560];
