// D3Q27 Lattice-Boltzmann collision shader

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

@group(0) @binding(0) var<storage, read_write> lattice: array<LatticePoint>;
@group(0) @binding(1) var<storage, read_write> temp: array<LatticePoint>;
@group(0) @binding(2) var<uniform> config: Config;

// D3Q27 constants
const Q: u32 = 27u;
const CS2: f32 = 1.0 / 3.0;

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

// D3Q27 weights
const WEIGHTS = array<f32, 27>(
    8.0/27.0,                    // 0
    2.0/27.0, 2.0/27.0, 2.0/27.0, 2.0/27.0, 2.0/27.0, 2.0/27.0,  // 1-6
    1.0/54.0, 1.0/54.0, 1.0/54.0, 1.0/54.0,  // 7-10
    1.0/54.0, 1.0/54.0, 1.0/54.0, 1.0/54.0,  // 11-14
    1.0/54.0, 1.0/54.0, 1.0/54.0, 1.0/54.0,  // 15-18
    1.0/216.0, 1.0/216.0, 1.0/216.0, 1.0/216.0,  // 19-22
    1.0/216.0, 1.0/216.0, 1.0/216.0, 1.0/216.0,  // 23-26
);

fn equilibrium_distribution(direction: u32, density: f32, velocity: array<f32, 3>) -> f32 {
    let weight = WEIGHTS[direction];
    let c = VELOCITIES[direction];
    
    // Dot product of velocity and lattice velocity
    let cu = f32(c[0]) * velocity[0] + f32(c[1]) * velocity[1] + f32(c[2]) * velocity[2];
    
    // Velocity magnitude squared
    let u2 = velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];
    
    // Ensure inputs are valid
    if (density <= 0.0 || abs(u2) > 1.0) {
        return weight * 1.0; // Return a safe default
    }
    
    // Equilibrium distribution function
    let eq = weight * density * (1.0 + cu / CS2 + 
                              cu * cu / (2.0 * CS2 * CS2) - 
                              u2 / (2.0 * CS2));
    
    // Ensure result is valid
    if (eq < 0.0 || eq != eq) { // Check for NaN
        return weight * density / 27.0; // Return uniform distribution
    }
    
    return eq;
}

fn calculate_macroscopic(idx: u32) {
    // Calculate density
    var density = 0.0;
    for (var i = 0u; i < Q; i++) {
        density += lattice[idx].f[i];
    }
    
    // Calculate velocity
    var velocity = array<f32, 3>(0.0, 0.0, 0.0);
    for (var i = 0u; i < Q; i++) {
        let c = VELOCITIES[i];
        velocity[0] += lattice[idx].f[i] * f32(c[0]);
        velocity[1] += lattice[idx].f[i] * f32(c[1]);
        velocity[2] += lattice[idx].f[i] * f32(c[2]);
    }
    
    // Ensure density is valid
    if (density <= 1e-10 || density != density) { // Check for NaN
        density = 1.0;
        velocity = array<f32, 3>(0.0, 0.0, 0.0);
    } else {
        velocity[0] /= density;
        velocity[1] /= density;
        velocity[2] /= density;
        
        // Clamp velocity to reasonable range
        let vel_mag = sqrt(velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2]);
        if (vel_mag > 0.3) { // Max Mach number around 0.3
            let scale = 0.3 / vel_mag;
            velocity[0] *= scale;
            velocity[1] *= scale;
            velocity[2] *= scale;
        }
    }
    
    lattice[idx].density = density;
    lattice[idx].velocity = velocity;
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
    
    // Handle different node types
    switch (lattice[idx].node_type) {
        case 0u: { // Fluid nodes - BGK collision
            // Calculate macroscopic quantities
            calculate_macroscopic(idx);
            
            // BGK collision
            let omega = 1.0 / config.tau;
            
            for (var i = 0u; i < Q; i++) {
                let f_eq = equilibrium_distribution(i, lattice[idx].density, lattice[idx].velocity);
                temp[idx].f[i] = lattice[idx].f[i] + omega * (f_eq - lattice[idx].f[i]);
            }
            
            temp[idx].density = lattice[idx].density;
            temp[idx].velocity = lattice[idx].velocity;
        }
        case 2u: { // Inlet nodes - prescribed velocity
            let inlet_vel = array<f32, 3>(
                config.inlet_velocity.x,
                config.inlet_velocity.y,
                config.inlet_velocity.z
            );
            
            // Set equilibrium distribution at inlet
            for (var i = 0u; i < Q; i++) {
                temp[idx].f[i] = equilibrium_distribution(i, config.density, inlet_vel);
            }
            
            temp[idx].density = config.density;
            temp[idx].velocity = inlet_vel;
        }
        default: { // Solid, outlet, and other nodes - copy unchanged
            for (var i = 0u; i < Q; i++) {
                temp[idx].f[i] = lattice[idx].f[i];
            }
            temp[idx].density = lattice[idx].density;
            temp[idx].velocity = lattice[idx].velocity;
        }
    }
    
    // Copy node type and padding
    temp[idx].node_type = lattice[idx].node_type;
    temp[idx].padding = lattice[idx].padding;
}
