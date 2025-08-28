use winit::keyboard::KeyCode;
use super::camera::Camera;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    Orbital,
    Fps,
}

pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    mode: CameraMode,
    // Contrôles clavier
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    // Pour FPS
    pitch: f32,
    yaw: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            sensitivity: 0.002, // Sensibilité souris pour FPS
            mode: CameraMode::Orbital,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    pub fn set_mode(&mut self, mode: CameraMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> CameraMode {
        self.mode
    }

    // Gestion des mouvements de souris pour le mode FPS
    pub fn handle_mouse_movement(&mut self, delta_x: f64, delta_y: f64) {
        if self.mode == CameraMode::Fps {
            self.yaw += delta_x as f32 * self.sensitivity;
            self.pitch -= delta_y as f32 * self.sensitivity;
            
            // Limiter le pitch pour éviter le gimbal lock
            self.pitch = self.pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.1, std::f32::consts::FRAC_PI_2 - 0.1);
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, is_pressed: bool) -> bool {
        match key {
            KeyCode::KeyT => {
                if is_pressed {
                    // Toggle entre modes orbital et FPS
                    self.mode = match self.mode {
                        CameraMode::Orbital => CameraMode::Fps,
                        CameraMode::Fps => CameraMode::Orbital,
                    };
                }
                true
            }
            KeyCode::Space => {
                self.is_up_pressed = is_pressed;
                true
            }
            KeyCode::ShiftLeft => {
                self.is_down_pressed = is_pressed;
                true
            }
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        match self.mode {
            CameraMode::Orbital => self.update_orbital_camera(camera),
            CameraMode::Fps => self.update_fps_camera(camera),
        }
    }

    fn update_orbital_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.length();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.length();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }

    fn update_fps_camera(&self, camera: &mut Camera) {        
        // Calculer la direction avant basée sur yaw et pitch
        let front = glam::Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalize();

        let right = front.cross(camera.up).normalize();
        let actual_up = right.cross(front).normalize();

        // Mouvement
        if self.is_forward_pressed {
            camera.eye += front * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= front * self.speed;
        }
        if self.is_right_pressed {
            camera.eye += right * self.speed;
        }
        if self.is_left_pressed {
            camera.eye -= right * self.speed;
        }
        if self.is_up_pressed {
            camera.eye += actual_up * self.speed;
        }
        if self.is_down_pressed {
            camera.eye -= actual_up * self.speed;
        }

        // Mettre à jour la target pour regarder dans la direction avant
        camera.target = camera.eye + front;
        camera.up = actual_up;
    }
}

