use std::collections::HashMap;
use crate::orbital_compute::OrbitalCompute;
use crate::rendering;
use crate::rendering::{read_obj, Camera, CameraUniform, Object, Projection, Vertex};
use std::sync::Arc;
use cgmath::num_traits::real::Real;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::wgt::{CommandEncoderDescriptor, DeviceDescriptor, TextureDescriptor, TextureViewDescriptor};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;
use winit::window::Window;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipelines: Vec<wgpu::RenderPipeline>,
    compute: OrbitalCompute,
    objects: Vec<crate::rendering::Object>,
    depth_stencil: wgpu::Texture,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    voxel_scale_factor: f32,
    keys: HashMap<winit::keyboard::KeyCode, bool>,
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }).await?;

        let (device, queue) = adapter.request_device(&DeviceDescriptor {
            label: Some("GPU device 413"),
            required_features: wgpu::Features::POLYGON_MODE_LINE | wgpu::Features::POLYGON_MODE_POINT | wgpu::Features::DEPTH_CLIP_CONTROL | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS | wgpu::Features::IMMEDIATES,
            required_limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits {
                    max_compute_invocations_per_workgroup: 1024,
                    max_compute_workgroup_size_x: 256,
                    max_compute_workgroup_size_y: 256,
                    max_compute_workgroup_size_z: 64,
                    max_immediate_size: 16,
                    ..Default::default()
                }
            },
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }).await?;

        println!("Adapter Features: {}\n", adapter.features());
        println!("Device Features: {}\n", device.features());
        println!("Adapter Limits: {:?}\n", adapter.limits());
        println!("Device Limits: {:?}", device.limits());

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
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

        let camera = Camera::new(cgmath::Point3::new(0.0, 0.0, -2.0), cgmath::Deg(90.0), cgmath::Deg(0.0), size.width, size.height);
        let camera_uniform = CameraUniform::new(&camera);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }]
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0, resource: camera_buffer.as_entire_binding()
            }],
        });

        let mut pieplines = Vec::new();

        // Pipelines @_@
        {
            // Triangle
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("Triangle Shader Module"), source: wgpu::ShaderSource::Wgsl(include_str!("shader/shader.wgsl").into()) });

            let triangle_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Triangle Render Pipeline layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 4,
            });

            let triangle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Triangle Render Pipeline 413"),
                layout: Some(&triangle_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_triangle"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[crate::rendering::Vertex::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: true,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main_triangle"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });


            // Line
            let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Render Pipeline layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 0,
            });

            let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Line Render Pipeline 413"),
                layout: Some(&line_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_line"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[crate::rendering::Vertex::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Line,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main_line"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });


            // Point
            let point_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Point Render Pipeline layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 4,
            });

            let point_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Point Render Pipeline 413"),
                layout: Some(&point_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_point"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[crate::rendering::Vertex::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::PointList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Point,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main_point"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

            pieplines = vec![triangle_pipeline, line_pipeline, point_pipeline];
        }

        let mut objects = vec![read_obj("src/objects/renderercube.obj", &device).scale(0.01, &device, Some("renderercububububu"))];
        //objects.push(objects[0].scale(0.5, &device, Some("Scaled")).translate(-1.0, -0.5, -1.0, &device, Some("Translated")));

        //let objects = vec![];

        let depth_stencil = device.create_texture(&TextureDescriptor {
            label: Some("Depth Stencil"),
            size: wgpu::Extent3d {width: size.width, height: size.height, depth_or_array_layers: 1},
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Depth32Float],
        });

        let compute = OrbitalCompute::new(&device);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipelines: pieplines,
            compute,
            objects,
            depth_stencil,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            voxel_scale_factor: 1.0,
            keys: HashMap::new(),
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_stencil = self.device.create_texture(&TextureDescriptor {
                label: Some("Depth Stencil"),
                size: wgpu::Extent3d {width, height, depth_or_array_layers: 1},
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[wgpu::TextureFormat::Depth32Float],
            });

            self.camera.projection = Projection::new(width, height, cgmath::Deg(45.0), 0.1, 100.0);
            self.is_surface_configured = true;
        }
    }

    pub fn request_compute(&mut self, n: f32, l: f32, m: f32) {
        println!("STARING COMPUTE");
        let now = std::time::Instant::now();
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Compute Encoder") });
        self.queue.write_buffer(&self.compute.params_buffer, 0, bytemuck::cast_slice(&[n, l, m]));

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass Descriptor"), timestamp_writes: None });
            compute_pass.set_pipeline(&self.compute.pipeline);
            compute_pass.set_bind_group(0, Some(&self.compute.bind_group), &[]);
            compute_pass.dispatch_workgroups(10, 10, 10);
            compute_pass.on_submitted_work_done(|| println!("WORK DONE! :D"));
        }

        encoder.copy_buffer_to_buffer(&self.compute.buffer_out, 0, &self.compute.true_out, 0, self.compute.buffer_out.size());

        self.queue.submit(std::iter::once(encoder.finish()));

        let (sender, mut receiver) = futures_channel::oneshot::channel();

        self.compute.true_out.map_async(wgpu::MapMode::Read, .., move |result| sender.send(result).unwrap());
        self.device.poll(wgpu::PollType::wait_indefinitely());

        receiver.try_recv().unwrap().unwrap().unwrap();

        let result: Vec<f32> = self.compute.true_out.get_mapped_range(..).chunks_exact(4).map(|e| f32::from_le_bytes(<[u8; 4]>::try_from(e).unwrap())).collect();
        self.compute.true_out.unmap();
        println!("Computation finished in: {}", now.elapsed().as_millis());
        let max = match result.clone().into_iter().filter(|e| e.is_finite()).reduce(f32::max) {
            Some(a) => a,
            None => panic!("No results from shader!!!!!!!!!"),
        };

        println!("Max finished in: {}, with val {max}", now.elapsed().as_millis());
        let vert: Vec<Vertex> = result.iter().enumerate().filter(|e| *e.1 >= max / 10.0).map(|e| {
            let z = e.0 / 10000;
            let y = (e.0 - 10000 * z) / 100;
            let x = e.0 - 10000* z - 100 * y;
            let x_c = x as f32 / 50.0 - 1.0;
            let y_c = y as f32 / 50.0 - 1.0;
            let z_c = z as f32 / 50.0 - 1.0;
            [Vertex {position: [x_c, y_c, z_c-0.005], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c, y_c-0.005, z_c-0.005], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c, y_c, z_c], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c, y_c-0.005, z_c], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c-0.005, y_c, z_c-0.005], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c-0.005, y_c-0.005, z_c-0.005], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c-0.005, y_c, z_c], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]},
                Vertex {position: [x_c-0.005, y_c-0.005, z_c], color: [x_c + 0.5, y_c + 0.5, z_c + 0.5]}]
        }).flatten().collect();

        if vert.is_empty() {
            println!("DIDN'T RETURN ANY COMPUTATION");
            return;
        }

        let faces: Vec<u32> = (0..(vert.len() as u32) / 8).map(|i| [
            i*8+4, i*8+2, i*8,
            i*8+2, i*8+7, i*8+3,
            i*8+6, i*8+5, i*8+7,
            i*8+1, i*8+7, i*8+5,
            i*8, i*8+3, i*8+1,
            i*8+4, i*8+1, i*8+5,
            i*8+4, i*8+6, i*8+2,
            i*8+2, i*8+6, i*8+7,
            i*8+6, i*8+4, i*8+5,
            i*8+1, i*8+3, i*8+7,
            i*8, i*8+2, i*8+3,
            i*8+4, i*8, i*8+1]).flatten().collect();
        let x = Object::new(vert, faces, vec![0], &self.device, Some("Computed"));
        self.objects.push(x);
        println!("Pre-Processing finished in: {}", now.elapsed().as_millis());
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                anyhow::bail!("Lost device");
            }
        };
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        let depth = self.depth_stencil.create_view(&TextureViewDescriptor {
            label: Some("Depth Stencil View"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            aspect: Default::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Render Encoder 413") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass 413"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipelines[0]);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_immediates(0, &self.voxel_scale_factor.to_le_bytes());
            for obj in self.objects.iter() {
                render_pass.set_vertex_buffer(0, obj.vertex_buffer.slice(..));
                render_pass.set_index_buffer(obj.face_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..obj.faces.len() as u32, 0, 0..1);
            }
            // render_pass.set_pipeline(&self.render_pipelines[1]);
            // render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            // for obj in self.objects.iter() {
            //     render_pass.set_vertex_buffer(0, obj.vertex_buffer.slice(..));
            //     render_pass.set_index_buffer(obj.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            //     render_pass.draw_indexed(0..(obj.edges.len() as u32), 0, 0..1);
            // }

            // render_pass.set_pipeline(&self.render_pipelines[2]);
            // render_pass.set_immediates(0, &self.voxel_scale_factor.to_le_bytes());
            // render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            // for obj in self.objects.iter() {
            //     render_pass.set_vertex_buffer(0, obj.vertex_buffer.slice(..));
            //     render_pass.draw(0..obj.vertices.len() as u32, 0..1);
            // }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        return Ok(());
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        self.camera.update_camera(code, is_pressed);
        self.camera_uniform.update_view_proj(&self.camera);
        self.keys.insert(code, is_pressed);
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::Numpad5, true) => self.debug_log(),
            (KeyCode::Numpad6, true) => self.request_compute(4.0, 3.0, 1.0),
            _ => {}
        }
    }

    fn debug_log(&mut self) {
        println!("CAMERA: pitch: {}, yaw: {}, position: {:?}, view_projection: {:?}", self.camera.pitch.0, self.camera.yaw.0, self.camera.position, self.camera_uniform.view_proj);
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match (button, state.is_pressed()) {
            (MouseButton::Left, _) => self.camera.rotating = state.is_pressed(),
            (_, _) => {}
        }
    }

    pub fn handle_scroll(&mut self, delta: winit::event::MouseScrollDelta, phase: winit::event::TouchPhase) {
        match delta {
            MouseScrollDelta::LineDelta(_, dy) => {
                match self.keys.get(&KeyCode::ShiftLeft) {
                    Some(true) => self.voxel_scale_factor += dy / 100.0,
                    _ => {}
                }
            }
            MouseScrollDelta::PixelDelta(dy) => {
                match self.keys.get(&KeyCode::ShiftLeft) {
                    Some(true) => self.voxel_scale_factor += dy.y as f32 / 100.0,
                    _ => {}
                }
            }
        }
    }

    pub fn handle_mouse_movement(&mut self, dx: f64, dy: f64) {
        if self.camera.rotating {
            self.camera.yaw -= cgmath::Rad(dx as f32 / self.config.width as f32);
            self.camera.pitch += cgmath::Rad(dy as f32 / self.config.height as f32);
            self.camera_uniform.update_view_proj(&self.camera);
        }

        if self.camera.pitch < -cgmath::Rad(rendering::SAFE_FRAC_PI_2) {
            self.camera.pitch = -cgmath::Rad(rendering::SAFE_FRAC_PI_2);
        } else if self.camera.pitch > cgmath::Rad(rendering::SAFE_FRAC_PI_2) {
            self.camera.pitch = cgmath::Rad(rendering::SAFE_FRAC_PI_2);
        }
    }

    pub fn update(&mut self) {
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }
}