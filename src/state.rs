use std::sync::Arc;
use wgpu::{include_wgsl, Backends, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType, Color, ColorTargetState, ColorWrites, CompareFunction, ComputePipeline, ComputePipelineDescriptor, DepthStencilState, Extent3d, Face, Features, FragmentState, FrontFace, MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode, PowerPreference, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule, ShaderModuleDescriptor, ShaderSource, SurfaceConfiguration, TextureFormat, TextureUsages, Trace, VertexBufferLayout, VertexState};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::wgt::{CommandEncoderDescriptor, DeviceDescriptor, TextureDescriptor, TextureViewDescriptor};
use winit::event::{ElementState, MouseButton};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;
use winit::window::Window;
use crate::rendering;
use crate::rendering::{read_obj, Camera, CameraUniform};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipelines: Vec<wgpu::RenderPipeline>,
    compute_pipeline: wgpu::ComputePipeline,
    objects: Vec<crate::rendering::Object>,
    depth_stencil: wgpu::Texture,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
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

        let adapter = instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }).await?;

        let (device, queue) = adapter.request_device(&DeviceDescriptor {
            label: Some("GPU device 413"),
            required_features: Features::POLYGON_MODE_LINE | Features::POLYGON_MODE_POINT | Features::DEPTH_CLIP_CONTROL,
            required_limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            },
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: Trace::Off,
        }).await?;

        println!("Adapter Features: {}\n", adapter.features());
        println!("Device Features: {}", device.features());

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

        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }]
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0, resource: camera_buffer.as_entire_binding()
            }],
        });

        let mut pieplines = Vec::new();

        // Pipelines @_@
        {
            // Triangle
            let shader = device.create_shader_module(ShaderModuleDescriptor { label: Some("Triangle Shader Module"), source: ShaderSource::Wgsl(include_str!("shader/shader.wgsl").into()) });

            let triangle_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Render Pipeline layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 0,
            });

            let triangle_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Render Pipeline 413"),
                layout: Some(&triangle_pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_triangle"),
                    compilation_options: PipelineCompilationOptions { constants: &[], zero_initialize_workgroup_memory: false },
                    buffers: &[crate::rendering::Vertex::desc()],
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: true,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main_triangle"),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });


            // Line
            let line_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Render Pipeline layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 0,
            });

            let line_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Line Render Pipeline 413"),
                layout: Some(&line_pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_line"),
                    compilation_options: PipelineCompilationOptions { constants: &[], zero_initialize_workgroup_memory: false },
                    buffers: &[crate::rendering::Vertex::desc()],
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Line,
                    conservative: false,
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main_line"),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });


            // Point
            let point_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Point Render Pipeline layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

            let point_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Render Pipeline 413"),
                layout: Some(&point_pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_point"),
                    compilation_options: PipelineCompilationOptions { constants: &[], zero_initialize_workgroup_memory: false },
                    buffers: &[VertexBufferLayout {
                        array_stride: 0,
                        step_mode: Default::default(),
                        attributes: &[],
                    }],
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::PointList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Point,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main_point"),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

            pieplines = vec![triangle_pipeline, line_pipeline, point_pipeline];
        }

        // Compute
        let compute_shader = device.create_shader_module(ShaderModuleDescriptor { label: Some("Compute Shader Module"), source: ShaderSource::Wgsl(include_str!("shader/compute_shader.wgsl").into()) });

        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Compute Pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: None,
            module: &compute_shader,
            entry_point: Some("compute_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let objects = vec![read_obj("src/objects/behold.obj", &device)];

        let depth_stencil = device.create_texture(&TextureDescriptor {
            label: Some("Depth Stencil"),
            size: Extent3d {width: size.width, height: size.height, depth_or_array_layers: 1},
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[TextureFormat::Depth32Float],
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipelines: pieplines,
            compute_pipeline,
            objects,
            depth_stencil,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
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
            usage: Some(TextureUsages::RENDER_ATTACHMENT),
            aspect: Default::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Render Encoder 413") });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass 413"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color::GREEN),
                        store: wgpu::StoreOp::Store
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(Operations {
                        load: wgpu::LoadOp::Clear(0.0),
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
            for obj in self.objects.iter() {
                render_pass.set_vertex_buffer(0, obj.vertex_buffer.slice(..));
                render_pass.set_index_buffer(obj.face_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..(obj.faces.len() as u32), 0,0..1);
            }
            render_pass.set_pipeline(&self.render_pipelines[1]);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            for obj in self.objects.iter() {
                render_pass.set_vertex_buffer(0, obj.vertex_buffer.slice(..));
                render_pass.set_index_buffer(obj.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..(obj.edges.len() as u32), 0, 0..1);
            }

        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        return Ok(());
    }

    // impl State
    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        self.camera.update_camera(code, is_pressed);
        self.camera_uniform.update_view_proj(&self.camera);
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::Numpad5, true) => self.debug_log(),
            _ => {}
        }
    }

    fn debug_log(&self) {
        println!("CAMERA: pitch: {}, yaw: {}, position: {:?}, view_projection: {:?}", self.camera.pitch.0, self.camera.yaw.0, self.camera.position, self.camera_uniform.view_proj);
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match (button, state.is_pressed()) {
            (MouseButton::Left, _) => self.camera.rotating = state.is_pressed(),
            (_, _) => {}
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