// D3Q27 Lattice-Boltzmann streaming shader

struct LatticePoint {
    f: array<f32, 27>,           // Distribution functions
    density: f32,                // Macroscopic density
    velocity: array<f32, 3>,     // Macroscopic velocity
    node_type: u32,              // Node type (0: fluid, 1: solid, 2: inlet, 3: outlet)
    padding: array<u32, 3>,      // Padding for alignment
}

struct Config {
    domain_size: vec4<u32>,         // nx, ny, nz, padding - 16 bytes aligned
    tau: f32,                       // 4 bytes
    density: f32,                   // 4 bytes  
    padding1: vec2<f32>,            // 8 bytes - total 16 bytes for this group
    inlet_velocity: vec4<f32>,      // 16 bytes aligned
}

@group(0) @binding(0) var<storage, read> source: array<LatticePoint>;
@group(0) @binding(1) var<storage, read_write> dest: array<LatticePoint>;
@group(0) @binding(2) var<uniform> config: Config;

// D3Q27 velocities
const VELOCITIES = array<array<i32, 3>, 27>(
    array<i32, 3>(0, 0, 0),     // 0
    array<i32, 3>(1, 0, 0),     // 1
    array<i32, 3>(-1, 0, 0),    // 2
    array<i32, 3>(0, 1, 0),     // 3
    array<i32, 3>(0, -1, 0),    // 4
    array<i32, 3>(0, 0, 1),     // 5
    array<i32, 3>(0, 0, -1),    // 6
    array<i32, 3>(1, 1, 0),     // 7
    array<i32, 3>(1, -1, 0),    // 8
    array<i32, 3>(-1, 1, 0),    // 9
    array<i32, 3>(-1, -1, 0),   // 10
    array<i32, 3>(1, 0, 1),     // 11
    array<i32, 3>(1, 0, -1),    // 12
    array<i32, 3>(-1, 0, 1),    // 13
    array<i32, 3>(-1, 0, -1),   // 14
    array<i32, 3>(0, 1, 1),     // 15
    array<i32, 3>(0, 1, -1),    // 16
    array<i32, 3>(0, -1, 1),    // 17
    array<i32, 3>(0, -1, -1),   // 18
    array<i32, 3>(1, 1, 1),     // 19
    array<i32, 3>(1, 1, -1),    // 20
    array<i32, 3>(1, -1, 1),    // 21
    array<i32, 3>(1, -1, -1),   // 22
    array<i32, 3>(-1, 1, 1),    // 23
    array<i32, 3>(-1, 1, -1),   // 24
    array<i32, 3>(-1, -1, 1),   // 25
    array<i32, 3>(-1, -1, -1),  // 26
);

fn get_source_index(x: u32, y: u32, z: u32, direction: u32) -> u32 {
    // For pull-based streaming, we need to find where f_i came FROM
    // If f_i streams from x-c_i to x, then to get f_i at x, we pull from x-c_i
    let c = VELOCITIES[direction];
    let nx = i32(x) - c[0];  // Note: MINUS c_i (opposite direction)
    let ny = i32(y) - c[1];
    let nz = i32(z) - c[2];
    
    // Handle boundary conditions with proper clamping
    var new_x = nx;
    var new_y = ny;
    var new_z = nz;
    
    // Apply periodic or reflective boundary conditions where appropriate
    if (new_x < 0) { 
        new_x = 0; // Reflective at inlet
    }
    if (new_x >= i32(config.domain_size.x)) { 
        new_x = i32(config.domain_size.x) - 1; // Reflective at outlet
    }
    if (new_y < 0) { 
        new_y = 0; // Reflective at walls
    }
    if (new_y >= i32(config.domain_size.y)) { 
        new_y = i32(config.domain_size.y) - 1; // Reflective at walls
    }
    if (new_z < 0) { 
        new_z = 0; // Reflective at walls
    }
    if (new_z >= i32(config.domain_size.z)) { 
        new_z = i32(config.domain_size.z) - 1; // Reflective at walls
    }
    
    return u32(new_x) + u32(new_y) * config.domain_size.x + u32(new_z) * config.domain_size.x * config.domain_size.y;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;
    
    if (x >= config.domain_size.x || y >= config.domain_size.y || z >= config.domain_size.z) {
        return;
    }
    
    let idx = x + y * config.domain_size.x + z * config.domain_size.x * config.domain_size.y;
    
    // Copy node properties
    dest[idx].density = source[idx].density;
    dest[idx].velocity = source[idx].velocity;
    dest[idx].node_type = source[idx].node_type;
    dest[idx].padding = source[idx].padding;
    
    // Streaming step: f_i(x, t + 1) = f_i(x - c_i, t) (pull-based)
    for (var i = 0u; i < 27u; i++) {
        let source_idx = get_source_index(x, y, z, i);
        dest[idx].f[i] = source[source_idx].f[i];
    }
}
