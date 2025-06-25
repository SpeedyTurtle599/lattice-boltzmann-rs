use wgpu;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== GPU Memory Diagnostics ===\n");
    
    // Initialize wgpu
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    // Get adapter
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find suitable adapter");
    
    let adapter_info = adapter.get_info();
    println!("GPU Information:");
    println!("  Name: {}", adapter_info.name);
    println!("  Vendor: {:?}", adapter_info.vendor);
    println!("  Device Type: {:?}", adapter_info.device_type);
    println!("  Backend: {:?}", adapter_info.backend);
    println!();
    
    // Get device limits
    let limits = adapter.limits();
    println!("GPU Limits:");
    println!("  Max Buffer Size: {} bytes ({:.2} GB)", 
             limits.max_buffer_size, 
             limits.max_buffer_size as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("  Max Storage Buffer Binding Size: {} bytes ({:.2} GB)", 
             limits.max_storage_buffer_binding_size,
             limits.max_storage_buffer_binding_size as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("  Max Uniform Buffer Binding Size: {} bytes ({:.2} MB)", 
             limits.max_uniform_buffer_binding_size,
             limits.max_uniform_buffer_binding_size as f64 / 1024.0 / 1024.0);
    println!("  Max Compute Workgroup Storage Size: {} bytes ({:.2} KB)", 
             limits.max_compute_workgroup_storage_size,
             limits.max_compute_workgroup_storage_size as f64 / 1024.0);
    println!("  Max Compute Workgroups per Dimension: {}", limits.max_compute_workgroups_per_dimension);
    println!("  Max Compute Workgroup Size X: {}", limits.max_compute_workgroup_size_x);
    println!("  Max Compute Workgroup Size Y: {}", limits.max_compute_workgroup_size_y);
    println!("  Max Compute Workgroup Size Z: {}", limits.max_compute_workgroup_size_z);
    println!("  Max Compute Invocations per Workgroup: {}", limits.max_compute_invocations_per_workgroup);
    println!();
    
    // Try to create device with high limits
    let features = wgpu::Features::empty();
    let high_limits = wgpu::Limits {
        max_buffer_size: 16_000_000_000, // 16GB as u64
        max_storage_buffer_binding_size: 4_000_000_000, // 4GB as u32 (near the limit)
        ..limits
    };
    
    println!("Attempting to create device with high limits...");
    match adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("High Limits Device"),
            required_features: features,
            required_limits: high_limits.clone(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        },
    ).await {
        Ok(_) => {
            println!("✅ SUCCESS: Device created with high limits!");
            println!("  Requested Max Buffer Size: {:.2} GB", 
                     high_limits.max_buffer_size as f64 / 1024.0 / 1024.0 / 1024.0);
            println!("  Requested Max Storage Buffer: {:.2} GB", 
                     high_limits.max_storage_buffer_binding_size as f64 / 1024.0 / 1024.0 / 1024.0);
        },
        Err(e) => {
            println!("❌ FAILED: Cannot create device with high limits");
            println!("  Error: {}", e);
            println!("  Falling back to default limits...");
        }
    }
    
    // Test with default limits
    println!("\nTesting with adapter default limits...");
    match adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Default Device"),
            required_features: features,
            required_limits: limits.clone(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        },
    ).await {
        Ok((device, _queue)) => {
            println!("✅ SUCCESS: Device created with default limits!");
            
            // Test large buffer creation
            let test_sizes = vec![
                256 * 1024 * 1024,      // 256MB
                512 * 1024 * 1024,      // 512MB  
                1024 * 1024 * 1024,     // 1GB
                2_000_000_000_u64,      // 2GB
                4_000_000_000_u64,      // 4GB
            ];
            
            println!("\nTesting buffer creation:");
            for &size in &test_sizes {
                let size_gb = size as f64 / 1024.0 / 1024.0 / 1024.0;
                // Note: wgpu create_buffer doesn't return Result, it panics on failure
                // So we'll just create the buffer and see if it succeeds
                let _buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("Test Buffer {:.1}GB", size_gb)),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                println!("  ✅ {:.1}GB buffer: SUCCESS", size_gb);
            }
        },
        Err(e) => {
            println!("❌ FAILED: Cannot create device with default limits");
            println!("  Error: {}", e);
        }
    }
    
    // Calculate what domain sizes are possible
    println!("\n=== Domain Size Calculations ===");
    let lattice_point_size = std::mem::size_of::<f32>() * 20; // Rough estimate for LatticePoint
    println!("Estimated LatticePoint size: {} bytes", lattice_point_size);
    
    let max_points = limits.max_buffer_size / lattice_point_size as u64;
    println!("Max points with default limits: {:.1}M", max_points as f64 / 1_000_000.0);
    
    // Suggest optimal domain sizes
    println!("\nSuggested high-resolution domains:");
    let target_sizes = vec![
        (400, 100, 100), // 4M points
        (300, 150, 100), // 4.5M points  
        (500, 100, 80),  // 4M points
        (600, 100, 70),  // 4.2M points
        (800, 80, 80),   // 5.12M points
    ];
    
    for (nx, ny, nz) in target_sizes {
        let points = nx * ny * nz;
        let memory_mb = points * lattice_point_size / (1024 * 1024);
        let fits = (points * lattice_point_size) <= limits.max_buffer_size as usize;
        let status = if fits { "✅" } else { "❌" };
        println!("  {} {}×{}×{}: {:.1}M points, {:.1}MB", 
                 status, nx, ny, nz, points as f64 / 1_000_000.0, memory_mb);
    }
    
    Ok(())
}
