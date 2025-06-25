use anyhow::Result;
use std::fs::File;
use std::io::Write;

/// Generate a simple example STL file with a cylinder obstacle
pub fn generate_cylinder_stl(filename: &str, radius: f32, height: f32, center: [f32; 3]) -> Result<()> {
    let mut file = File::create(filename)?;
    
    // Write STL header
    writeln!(file, "solid cylinder")?;
    
    let num_segments = 20;
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
    // Generate an example cylinder STL file
    // Domain: 100x50x50 grid with dx=dy=dz=0.01 => 1.0m x 0.5m x 0.5m physical domain
    // Place cylinder at x=0.4m (40% downstream), centered in y-z directions
    // Cylinder: radius 0.08m (8 grid cells), height 0.4m (40 grid cells) 
    // This ensures cylinder spans most of the z-direction and creates a visible obstruction
    generate_cylinder_stl("example_cylinder.stl", 0.08, 0.4, [0.4, 0.25, 0.25])?;
    
    println!("Generated example_cylinder.stl");
    println!("Domain: 1.0m x 0.5m x 0.5m (100x50x50 grid cells)");
    println!("Cylinder: radius=0.08m, height=0.4m, center=(0.4, 0.25, 0.25)");
    println!("Inlet: x=0 plane, Outlet: x=1.0 plane");
    println!("Inlet velocity: 0.1 m/s in +x direction");
    
    Ok(())
}
