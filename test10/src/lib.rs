use std::{iter, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Instant};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::js_sys;

use winit::{
    application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

// Abstraction pour le timing cross-platform
#[cfg(target_arch = "wasm32")]
type TimeStamp = f64;

#[cfg(not(target_arch = "wasm32"))]
type TimeStamp = Instant;

#[cfg(target_arch = "wasm32")]
fn now() -> TimeStamp {
    js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> TimeStamp {
    Instant::now()
}

#[cfg(target_arch = "wasm32")]
fn time_diff_ms(start: TimeStamp, end: TimeStamp) -> f64 {
    end - start
}

#[cfg(not(target_arch = "wasm32"))]
fn time_diff_ms(start: TimeStamp, end: TimeStamp) -> f64 {
    end.duration_since(start).as_millis() as f64
}

pub mod geometry {
    pub mod icosphere;
    pub mod kdtree3d;
    pub mod fbm;
}

mod camera;
mod stellar_system;
pub mod celestial_body;
pub mod time;
pub mod manager;

use celestial_body::planet::planet_geometry::{PlanetGeometry, PlanetVertex};
use celestial_body::planet::render_pipeline::planet_render_pipeline;
use celestial_body::star::render_pipeline::star_render_pipeline;
use celestial_body::geometry_loader::{CelestialBodyHandle, CelestialBodyGeometry};
use camera::{Camera, CameraUniform, CameraController};
use stellar_system::{CelestialBody, StellarSystem};
use camera::init::init_camera_scene;
use time::time::init_time_scene;
use manager::manager::Manager;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{celestial_body::star::star_geometry::StarGeometry, time::time::TimeUniformGroup};


pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: Vec<wgpu::RenderPipeline>,
    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    window: Arc<Window>,
    manager: Manager,
    pub time_uniform_group: TimeUniformGroup,
    // Pour la gestion de la souris FPS
    last_mouse_pos: Option<winit::dpi::PhysicalPosition<f64>>,
    mouse_pressed: bool,
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        // Utiliser une taille par défaut si la fenêtre n'est pas encore configurée
        let (width, height) = if size.width == 0 || size.height == 0 {
            (800, 600) // taille par défaut
        } else {
            (size.width, size.height)
        };

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off, // Trace path
            })
            .await
            .unwrap();

        log::info!("({:?})", adapter.get_info());

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let (
            camera, 
            camera_controller, 
            camera_uniform, 
            camera_buffer, 
            camera_bind_group_layout, 
            camera_bind_group
        ) = init_camera_scene(&device, &config);

        let time_uniform_group = init_time_scene(&device);

        let render_pipeline_planet = planet_render_pipeline(
            &device, 
            &[
                &camera_bind_group_layout,
            ], 
            &config
        );

        let render_pipeline_star = star_render_pipeline(
            &device, 
            &[
                &camera_bind_group_layout,
                &time_uniform_group.time_bgl
            ], 
            &config
        );

        let system = StellarSystem::new(glam::Vec3::new(0.0,0.0,0.0));

        let result: Vec<CelestialBodyHandle> = system.bodies.iter().enumerate().map(|(i, body)| {
            match body {
                CelestialBody::Star(star) => {
                    log::info!("STAR");
                CelestialBodyHandle::new(
                    CelestialBodyGeometry::Star(StarGeometry::new()),
                    star.position,
                    glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0_f32.to_radians()),
                    i as u32
                )},
                CelestialBody::Planet(planet) => {
                    log::info!("PLANET");
                CelestialBodyHandle::new(
                    CelestialBodyGeometry::Planet(PlanetGeometry::new()),
                    planet.position,
                    glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0_f32.to_radians()),
                    i as u32
                )},
            }
        }).collect();

        log::info!("Taille du Vec<PlanetHandle> : {}", result.len());

        let manager = Manager::new(result);


      Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline: vec![render_pipeline_planet, render_pipeline_star],
            camera,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            window,
            manager,
            time_uniform_group,
            last_mouse_pos: None,
            mouse_pressed: false,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.is_surface_configured = true;
            self.config.width = width;
            self.config.height = height;
            self.camera_uniform.aspect_ratio = width as f32 / height as f32;
            self.surface.configure(&self.device, &self.config);

            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
        }
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, pressed: bool) {
        if key == KeyCode::Escape && pressed {
            event_loop.exit();
        } else {
            self.camera_controller.handle_key(key, pressed);
        }
    }

    fn handle_mouse_input(&mut self, button: MouseButton, pressed: bool) {
        if button == MouseButton::Left {
            self.mouse_pressed = pressed;
        }
    }

    fn handle_mouse_motion(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        if let Some(last_pos) = self.last_mouse_pos {
            let delta_x = position.x - last_pos.x;
            let delta_y = position.y - last_pos.y;
            
            // En mode FPS, toujours appliquer le mouvement de souris
            // En mode orbital, seulement si le bouton gauche est pressé
            if self.camera_controller.get_mode() == camera::controller::CameraMode::Fps || self.mouse_pressed {
                self.camera_controller.handle_mouse_movement(delta_x, delta_y);
            }
        }
        self.last_mouse_pos = Some(position);
    }

    fn update(&mut self) {

        self.time_uniform_group.time_uniform.time += 0.001;

        self.queue.write_buffer(
            &self.time_uniform_group.time_buffer,
            0,
            bytemuck::cast_slice(&[self.time_uniform_group.time_uniform]),
        );

        // Optimisation mise en cache des Matrices et utilisation de timestamp pour reprendre sur element non visible non compute par frame

        for planet_instance in &mut self.manager.planet_instances {
            planet_instance.instance.update_rotation(0.01, 0.0);
            planet_instance.recompute_instance(&self.device);
        }

        self.camera_uniform.get_view_proj();
        let mat4 = CameraUniform::mat4_from_array(self.camera_uniform.get_view_proj());
        let planes = Camera::extract_frustum_planes(&mat4);
        self.manager.set_planes(planes);
        self.manager.check_visibility_cluster(&self.device);

        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.manager.render_visible_object(
                &mut render_pass, 
                &self.render_pipeline, 
                &self.camera_bind_group,
                &self.time_uniform_group.time_bg
            );
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}











pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    last_render_time: TimeStamp,
    target_frame_time_ms: f64,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            last_render_time: now(),
            target_frame_time_ms: 1000.0 / 60.0, // 60 FPS
        }
    }

    fn should_render(&self) -> bool {
        let current_time = now();
        time_diff_ms(self.last_render_time, current_time) >= self.target_frame_time_ms
    }

    fn update_render_time(&mut self) {
        self.last_render_time = now();
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                // Vérifier la limitation FPS avant tout traitement
                if !self.should_render() {
                    if let Some(state) = &self.state {
                        state.window.request_redraw();
                    }
                    return;
                }

                let state = match &mut self.state {
                    Some(canvas) => canvas,
                    None => return,
                };

                state.update();
                match state.render() {
                    Ok(_) => {
                        self.update_render_time();
                    }
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }

                // let val: Instant = now() + 16.0;
                // event_loop.set_control_flow(
                //     winit::event_loop::ControlFlow::WaitUntil(
                //         now() + 16.0,
                //     ),
                // );

            }
            _ => {
                let state = match &mut self.state {
                    Some(canvas) => canvas,
                    None => return,
                };

                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(size) => state.resize(size.width, size.height),
                    WindowEvent::MouseInput { state: button_state, button, .. } => {
                        state.handle_mouse_input(button, button_state.is_pressed());
                    },
                    WindowEvent::CursorMoved { position, .. } => {
                        state.handle_mouse_motion(position);
                    },
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(code),
                                state: key_state,
                                ..
                            },
                        ..
                    } => state.handle_key(event_loop, code, key_state.is_pressed()),
                    _ => {}
                }
            }
        }

        // Demander un nouveau redraw pour maintenir la boucle de rendu
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    log::info!("Salut je me lance en prems");
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
// #[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();


    Ok(())
}