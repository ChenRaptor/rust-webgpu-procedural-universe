use std::{iter, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Instant};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::js_sys;

use wgpu::util::DeviceExt;
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

use celestial_body::planet::planet_geometry::{PlanetGeometry, PlanetHandle, PlanetVertex};
use celestial_body::planet::render_pipeline::planet_render_pipeline;
use camera::{Camera, CameraUniform, CameraController, Plane};
use stellar_system::{CelestialBody, StellarSystem};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

struct Manager {
    pub planet_instances: Vec<PlanetHandle>,
    pub buffer_loader: Vec<u32>,
    planes: [Plane; 6],
    pub in_computing: bool,
    pub id_in_computing: u32
}

impl Manager {

    pub fn new(planets: Vec<PlanetHandle>) -> Self {
        Manager {
            planet_instances: planets,
            buffer_loader: Vec::new(),
            planes: [Plane::default(); 6],
            in_computing: false,
            id_in_computing: 0
        }
    }

    fn set_planes(&mut self, planes: [Plane; 6])
    {
        self.planes = planes;
    }

    fn check_visibility_cluster(&mut self, device: &wgpu::Device)
    {
        for planet_instance in &mut self.planet_instances {
            let mut visible = true;
            for plane in &self.planes {
                if plane.normal.dot(planet_instance.instance.position) + plane.d < -1.5 {
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

    fn render_visible_object(&mut self, render_pass: &mut wgpu::RenderPass)
    {
        for planet_instance in &mut self.planet_instances {
            if planet_instance.is_visible && planet_instance.is_ready()
            {
                if let (Some(vb), Some(ib), Some(jo)) = (&planet_instance.vertex_buffer, &planet_instance.index_buffer, &planet_instance.instance_buffer) {
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_vertex_buffer(1, jo.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..planet_instance.num_indices, 0, 0..1);
                }
            }
        }
    }
}













#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
}

impl ModelUniform {
    fn new() -> Self {
        Self {
            model: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update_rotation(&mut self, angle: f32) {
        let rotation = glam::Mat4::from_rotation_y(angle); // rotation sur Y
        self.model = rotation.to_cols_array_2d();
    }
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    model_uniform: ModelUniform,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
    rotation_angle: f32,
    window: Arc<Window>,
    manager: Manager
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

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
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let camera = Camera::new(config.width as f32 / config.height as f32);
        let camera_controller = CameraController::new(0.2);

        let mut camera_uniform = CameraUniform::new();
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
                    visibility: wgpu::ShaderStages::VERTEX,
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

        let mut model_uniform = ModelUniform::new();
        model_uniform.update_rotation(0.0); // initialement pas de rotation

        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: bytemuck::cast_slice(&[model_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("model_bind_group_layout"),
        });

        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
            label: Some("model_bind_group"),
        });

        let render_pipeline = planet_render_pipeline(
            &device, 
            &[
                &camera_bind_group_layout,
                &model_bind_group_layout,
            ], 
            &config
        );

        let system = StellarSystem::new(glam::Vec3::new(0.0,0.0,0.0));

        let result: Vec<PlanetHandle> = system.bodies.iter().enumerate().map(|(i, body)| {
            match body {
                CelestialBody::Star(star) => PlanetHandle::new(
                    PlanetGeometry::new(),
                    star.position,
                    glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0_f32.to_radians()),
                    i as u32
                ),
                CelestialBody::Planet(planet) => PlanetHandle::new(
                    PlanetGeometry::new(),
                    planet.position,
                    glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0_f32.to_radians()),
                    i as u32
                ),
            }
        }).collect();

        log::info!("Taille du Vec<PlanetHandle> : {}", result.len());

        let mut manager = Manager::new(result);


      Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            camera,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            model_uniform,
            model_buffer,
            model_bind_group,
            rotation_angle: 0.0,
            window,
            manager,
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

    fn update(&mut self) {
  

        self.camera_uniform.get_view_proj();
        let mat4 = CameraUniform::mat4_from_array(self.camera_uniform.get_view_proj());
        let planes = Camera::extract_frustum_planes(&mat4);
        // let mut manager = Manager::new(planes);
        self.manager.set_planes(planes);
        self.manager.check_visibility_cluster(&self.device);
        
        self.rotation_angle += 0.01; // vitesse de rotation
        self.model_uniform.update_rotation(self.rotation_angle);
        self.queue.write_buffer(
            &self.model_buffer,
            0,
            bytemuck::cast_slice(&[self.model_uniform]),
        );
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

            render_pass.set_pipeline(&self.render_pipeline);
            // render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.model_bind_group, &[]);


            self.manager.render_visible_object(&mut render_pass);
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
                    WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
                        (MouseButton::Left, true) => {}
                        (MouseButton::Left, false) => {}
                        _ => {}
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