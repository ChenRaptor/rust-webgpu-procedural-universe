
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: glam::Mat4 = glam::Mat4::from_cols(
    glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
    glam::Vec4::new(0.0, 1.0, 0.0, 0.0),
    glam::Vec4::new(0.0, 0.0, 0.5, 0.0),
    glam::Vec4::new(0.0, 0.0, 0.5, 1.0),
);

#[derive(Copy, Clone)]
pub struct Plane {
    pub normal: glam::Vec3,
    pub d: f32,
}

impl Plane {
    pub fn default() -> Self
    {
        Plane {
            normal: glam::Vec3::ZERO,
            d: 0.0
        }
    }
}

pub struct Camera {
    pub eye: glam::Vec3,
    pub target: glam::Vec3,
    pub up: glam::Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            eye: glam::Vec3::new(0.0, 0.0, 8.0),
            target: glam::Vec3::new(0.0, 0.0, 0.0),
            up: glam::Vec3::Y,
            aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = glam::Mat4::perspective_rh_gl(
            self.fovy.to_radians(),
            self.aspect,
            self.znear,
            self.zfar,
        );
        // let view_proj = proj * view;
        // log::info!("{:?}", view_proj);
        proj * view
    }

    /// Extrait les 6 plans du frustum à partir de la matrice view-projection (déjà transformée avec OPENGL_TO_WGPU_MATRIX)
    pub fn extract_frustum_planes(view_proj: &glam::Mat4) -> [Plane; 6] {
        // La matrice est en column-major, donc on transpose pour accéder aux lignes
        let m = view_proj.to_cols_array_2d();
        let row = |i| glam::Vec4::new(m[0][i], m[1][i], m[2][i], m[3][i]);
        let planes = [
            row(3) + row(0), // left
            row(3) - row(0), // right
            row(3) + row(1), // bottom
            row(3) - row(1), // top
            row(3) + row(2), // near
            row(3) - row(2), // far
        ];
        planes.map(|p| {
            let n = glam::Vec3::new(p.x, p.y, p.z);
            let l = n.length();
            Plane { normal: n / l, d: p.w / l }
        })
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        let mat = OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix();
        self.view_proj = mat.to_cols_array_2d();
    }

    pub fn get_view_proj(&self) -> [[f32; 4]; 4] {
        self.view_proj
    }

    pub fn mat4_from_array(m: [[f32; 4]; 4]) -> glam::Mat4 {
        glam::Mat4::from_cols(
            glam::Vec4::from(m[0]),
            glam::Vec4::from(m[1]),
            glam::Vec4::from(m[2]),
            glam::Vec4::from(m[3]),
        )
    }
}