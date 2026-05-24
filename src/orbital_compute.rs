use wgpu::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, ComputePipelineDescriptor, PipelineCompilationOptions, PipelineLayoutDescriptor, ShaderModuleDescriptor, ShaderSource};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub struct OrbitalCompute {
    pub pipeline: wgpu::ComputePipeline,
    pub data_in: Vec<u32>,
    pub buffer_in: wgpu::Buffer,
    pub data_out: Vec<f32>,
    pub buffer_out: wgpu::Buffer,
    pub true_out: wgpu::Buffer,
    pub params_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl OrbitalCompute {
    pub fn new(device: &wgpu::Device) -> Self {
        let compute_in_data: Vec<u32> = (0..100*100*100).collect();
        let compute_out_data: Vec<f32> = vec![];
        let params: Vec<f32> = vec![1.0, 0.0, 0.0];

        let compute_in_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Compute In Buffer"),
            contents: bytemuck::cast_slice(&*compute_in_data),
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        });

        let compute_out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Out Buffer"),
            size: compute_in_buffer.size(),
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let true_out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Map Buffer"),
            size: compute_in_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbital Params Buffer"),
            contents: bytemuck::cast_slice(&*params),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let compute_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }]
        });

        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0, resource: compute_in_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1, resource: compute_out_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2, resource: params_buffer.as_entire_binding(),
                }],
        });

        // Compute
        let compute_shader = device.create_shader_module(ShaderModuleDescriptor { label: Some("Compute Shader Module"), source: ShaderSource::Wgsl(include_str!("shader/compute_shader.wgsl").into()) });

        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Compute Pipeline layout"),
            bind_group_layouts: &[Some(&compute_bind_group_layout)],
            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("compute_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        return Self {
            pipeline: compute_pipeline,
            data_in: compute_in_data,
            buffer_in: compute_in_buffer,
            data_out: compute_out_data,
            buffer_out: compute_out_buffer,
            true_out: true_out_buffer,
            params_buffer,
            bind_group: compute_bind_group,
        }
    }
}