use anyhow::Result;
use std::fs::File;
use std::io::Write;

/// Generate a simple example STL file with a cylinder obstacle
pub fn generate_cylinder_stl(filename: &str, radius: f32, height: f32, center: [f32; 3]) -> Result<()> {
    let mut file = File::create(filename)?;
    
    // Write STL header
    writeln!(file, "solid cylinder")?;
    
    let num_segments = 50;
    let angle_step = 2.0 * std::f32::consts::PI / num_segments as f32;
    
    // Generate cylinder surface
    for i in 0..num_segments {
        let angle1 = i as f32 * angle_step;
        let angle2 = ((i + 1) % num_segments) as f32 * angle_step;
        
        let x1 = center[0] + radius * angle1.cos();
        let y1 = center[1] + radius * angle1.sin();
        let x2 = center[0] + radius * angle2.cos();
        let y2 = center[1] + radius * angle2.sin();
        
        let z_bottom = center[2] - height / 2.0;
        let z_top = center[2] + height / 2.0;
        
        // Side faces (two triangles per segment)
        // Triangle 1
        writeln!(file, "  facet normal {} {} {}", 
                (angle1.cos() + angle2.cos()) / 2.0,
                (angle1.sin() + angle2.sin()) / 2.0,
                0.0)?;
        writeln!(file, "    outer loop")?;
        writeln!(file, "      vertex {} {} {}", x1, y1, z_bottom)?;
        writeln!(file, "      vertex {} {} {}", x2, y2, z_bottom)?;
        writeln!(file, "      vertex {} {} {}", x1, y1, z_top)?;
        writeln!(file, "    endloop")?;
        writeln!(file, "  endfacet")?;
        
        // Triangle 2
        writeln!(file, "  facet normal {} {} {}", 
                (angle1.cos() + angle2.cos()) / 2.0,
                (angle1.sin() + angle2.sin()) / 2.0,
                0.0)?;
        writeln!(file, "    outer loop")?;
        writeln!(file, "      vertex {} {} {}", x2, y2, z_bottom)?;
        writeln!(file, "      vertex {} {} {}", x2, y2, z_top)?;
        writeln!(file, "      vertex {} {} {}", x1, y1, z_top)?;
        writeln!(file, "    endloop")?;
        writeln!(file, "  endfacet")?;
        
        // Bottom cap triangle
        writeln!(file, "  facet normal 0.0 0.0 -1.0")?;
        writeln!(file, "    outer loop")?;
        writeln!(file, "      vertex {} {} {}", center[0], center[1], z_bottom)?;
        writeln!(file, "      vertex {} {} {}", x1, y1, z_bottom)?;
        writeln!(file, "      vertex {} {} {}", x2, y2, z_bottom)?;
        writeln!(file, "    endloop")?;
        writeln!(file, "  endfacet")?;
        
        // Top cap triangle
        writeln!(file, "  facet normal 0.0 0.0 1.0")?;
        writeln!(file, "    outer loop")?;
        writeln!(file, "      vertex {} {} {}", center[0], center[1], z_top)?;
        writeln!(file, "      vertex {} {} {}", x2, y2, z_top)?;
        writeln!(file, "      vertex {} {} {}", x1, y1, z_top)?;
        writeln!(file, "    endloop")?;
        writeln!(file, "  endfacet")?;
    }
    
    writeln!(file, "endsolid cylinder")?;
    
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // Parse command line arguments
    let mut domain_x = 200u32;
    let mut domain_y = 100u32; 
    let mut domain_z = 100u32;
    let mut dx = 0.0005f32;
    let mut output = "example_cylinder.stl".to_string();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--domain-x" => {
                if i + 1 < args.len() {
                    domain_x = args[i + 1].parse().unwrap_or(200);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--domain-y" => {
                if i + 1 < args.len() {
                    domain_y = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--domain-z" => {
                if i + 1 < args.len() {
                    domain_z = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--dx" => {
                if i + 1 < args.len() {
                    dx = args[i + 1].parse().unwrap_or(0.0005);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    
    // Calculate physical domain dimensions
    let domain_x_m = domain_x as f32 * dx;
    let domain_y_m = domain_y as f32 * dx;  // Use dx for all dimensions to keep cubic cells
    let domain_z_m = domain_z as f32 * dx;
    
    // Optimal positioning for vortex shedding:
    // - Cylinder at 30% downstream from inlet
    // - Centered in Y and Z directions
    // - Diameter scales with domain resolution (aim for ~16-20 grid cells)
    
    let cylinder_center = [
        domain_x_m * 0.3,           // 30% from inlet
        domain_y_m * 0.5,           // Centered in Y
        domain_z_m * 0.5            // Centered in Z
    ];
    
    // Scale cylinder radius based on domain resolution
    // For ultra-high-res (800x100x100), aim for ~20 grid cells diameter
    let target_diameter_cells = if domain_x >= 800 { 20.0 } else { 16.0 };
    let cylinder_radius = (target_diameter_cells * dx) / 2.0;
    let cylinder_height = domain_z_m * 0.8;  // Span 80% of domain height
    
    generate_cylinder_stl(&output, cylinder_radius, cylinder_height, cylinder_center)?;
    
    // Calculate memory usage estimate
    let total_cells = domain_x as u64 * domain_y as u64 * domain_z as u64;
    let memory_mb = (total_cells * 9 * 4) as f64 / (1024.0 * 1024.0);  // 9 f32s per cell
    
    println!("Generated {} for vortex shedding simulation:", output);
    println!("  Domain: {}×{}×{} grid ({:.3}m × {:.3}m × {:.3}m)", 
             domain_x, domain_y, domain_z, domain_x_m, domain_y_m, domain_z_m);
    println!("  Grid resolution: {:.1}mm (dx=dy=dz={:.6}m)", dx * 1000.0, dx);
    println!("  Cylinder center: [{:.4}, {:.4}, {:.4}]m", 
             cylinder_center[0], cylinder_center[1], cylinder_center[2]);
    println!("  Cylinder diameter: {:.1}mm ({:.1} grid cells)", 
             cylinder_radius * 2000.0, (cylinder_radius * 2.0 / dx));
    println!("  Estimated memory usage: ~{:.1}MB", memory_mb);
    
    Ok(())
}
