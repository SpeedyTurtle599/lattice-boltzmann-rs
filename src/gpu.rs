use wgpu::util::DeviceExt;
use anyhow::Result;
use crate::{config::Config, lattice::LatticePoint};

pub struct GPUContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    
    // Compute shaders
    collision_pipeline: wgpu::ComputePipeline,
    streaming_pipeline: wgpu::ComputePipeline,
    boundary_pipeline: wgpu::ComputePipeline,
    
    // Buffers
    lattice_buffer: wgpu::Buffer,
    temp_buffer: wgpu::Buffer,
    config_buffer: wgpu::Buffer,
    
    // Bind groups
    collision_bind_group: wgpu::BindGroup,
    streaming_bind_group: wgpu::BindGroup,
    boundary_bind_group: wgpu::BindGroup,
    
    // Dimensions
    nx: u32,
    ny: u32,
    nz: u32,
}

impl GPUContext {
    pub async fn new(config: &Config) -> Result<Self> {
        // Initialize WGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable adapter"))?;
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;
        
        let nx = config.domain.nx as u32;
        let ny = config.domain.ny as u32;
        let nz = config.domain.nz as u32;
        
        // Create buffers
        let lattice_size = (nx * ny * nz) as wgpu::BufferAddress * std::mem::size_of::<LatticePoint>() as wgpu::BufferAddress;
        
        let lattice_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Lattice Buffer"),
            size: lattice_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Temporary Buffer"),
            size: lattice_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Configuration buffer
        let config_data = GPUConfig {
            domain_size: [nx, ny, nz, 0], // Fourth element is padding
            tau: config.calculate_tau(),
            density: config.physics.density,
            padding1: [0.0, 0.0],
            inlet_velocity: [
                config.physics.inlet_velocity[0],
                config.physics.inlet_velocity[1],
                config.physics.inlet_velocity[2],
                0.0, // padding
            ],
        };
        
        let config_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Config Buffer"),
            contents: bytemuck::cast_slice(&[config_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Load shaders
        let collision_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Collision Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/collision.wgsl").into()),
        });
        
        let streaming_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Streaming Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/streaming.wgsl").into()),
        });
        
        let boundary_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Boundary Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/boundary.wgsl").into()),
        });
        
        // Create bind group layouts for different shader types
        
        // Layout for collision and boundary shaders (both buffers read-write)
        let collision_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Collision Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Layout for streaming shader (first buffer read-only, second read-write)
        let streaming_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Streaming Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create pipeline layouts
        let collision_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Collision Pipeline Layout"),
            bind_group_layouts: &[&collision_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let streaming_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Streaming Pipeline Layout"),
            bind_group_layouts: &[&streaming_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipelines
        let collision_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Collision Pipeline"),
            layout: Some(&collision_pipeline_layout),
            module: &collision_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let streaming_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Streaming Pipeline"),
            layout: Some(&streaming_pipeline_layout),
            module: &streaming_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let boundary_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Boundary Pipeline"),
            layout: Some(&collision_pipeline_layout),
            module: &boundary_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        // Create bind groups
        let collision_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Collision Bind Group"),
            layout: &collision_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lattice_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: temp_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });
        
        let streaming_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Streaming Bind Group"),
            layout: &streaming_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: temp_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lattice_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });
        
        let boundary_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Boundary Bind Group"),
            layout: &collision_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lattice_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: temp_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });
        
        Ok(Self {
            device,
            queue,
            collision_pipeline,
            streaming_pipeline,
            boundary_pipeline,
            lattice_buffer,
            temp_buffer,
            config_buffer,
            collision_bind_group,
            streaming_bind_group,
            boundary_bind_group,
            nx,
            ny,
            nz,
        })
    }
    
    pub fn upload_lattice_data(&self, data: &[LatticePoint]) {
        self.queue.write_buffer(&self.lattice_buffer, 0, bytemuck::cast_slice(data));
    }
    
    pub async fn read_lattice_data(&self) -> Result<Vec<LatticePoint>> {
        let buffer_size = (self.nx * self.ny * self.nz) as usize * std::mem::size_of::<LatticePoint>();
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Copy Encoder"),
        });
        
        encoder.copy_buffer_to_buffer(&self.lattice_buffer, 0, &staging_buffer, 0, buffer_size as u64);
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        receiver.await??;
        
        let data = buffer_slice.get_mapped_range();
        let result: Vec<LatticePoint> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        staging_buffer.unmap();
        
        Ok(result)
    }
    
    pub fn step(&self) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LBM Step Encoder"),
        });
        
        let workgroup_size = 8;
        let dispatch_x = (self.nx + workgroup_size - 1) / workgroup_size;
        let dispatch_y = (self.ny + workgroup_size - 1) / workgroup_size;
        let dispatch_z = (self.nz + workgroup_size - 1) / workgroup_size;
        
        // Collision step
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Collision Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.collision_pipeline);
            compute_pass.set_bind_group(0, &self.collision_bind_group, &[]);
            compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, dispatch_z);
        }
        
        // Streaming step
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Streaming Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.streaming_pipeline);
            compute_pass.set_bind_group(0, &self.streaming_bind_group, &[]);
            compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, dispatch_z);
        }
        
        // Boundary conditions
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Boundary Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.boundary_pipeline);
            compute_pass.set_bind_group(0, &self.boundary_bind_group, &[]);
            compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, dispatch_z);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GPUConfig {
    domain_size: [u32; 4],      // nx, ny, nz, padding - 16 bytes aligned
    tau: f32,                   // 4 bytes
    density: f32,               // 4 bytes
    padding1: [f32; 2],         // 8 bytes - total 16 bytes for this group
    inlet_velocity: [f32; 4],   // 16 bytes aligned
}
