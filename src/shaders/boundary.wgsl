// D3Q27 Lattice-Boltzmann boundary conditions shader

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

// Opposite directions for bounce-back
const OPPOSITE = array<u32, 27>(
    0u,  // Center stays the same
    2u, 1u, 4u, 3u, 6u, 5u,  // Face opposites
    9u, 8u, 7u, 10u, 13u, 12u, 11u, 14u, 17u, 16u, 15u, 18u,  // Edge opposites
    26u, 25u, 24u, 23u, 22u, 21u, 20u, 19u,  // Corner opposites
);

fn equilibrium_distribution(direction: u32, density: f32, velocity: array<f32, 3>) -> f32 {
    let weight = WEIGHTS[direction];
    let c = VELOCITIES[direction];
    
    // Dot product of velocity and lattice velocity
    let cu = f32(c[0]) * velocity[0] + f32(c[1]) * velocity[1] + f32(c[2]) * velocity[2];
    
    // Velocity magnitude squared
    let u2 = velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];
    
    // Equilibrium distribution function
    return weight * density * (1.0 + cu / CS2 + 
                              cu * cu / (2.0 * CS2 * CS2) - 
                              u2 / (2.0 * CS2));
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
    let node_type = lattice[idx].node_type;
    
    // Handle boundary conditions based on node type
    switch (node_type) {
        case 1u: { // Solid node - bounce-back
            // Full bounce-back boundary condition
            for (var i = 0u; i < Q; i++) {
                let opposite = OPPOSITE[i];
                lattice[idx].f[i] = lattice[idx].f[opposite];
            }
            lattice[idx].velocity = array<f32, 3>(0.0, 0.0, 0.0);
            lattice[idx].density = 1.0;
        }
        case 3u: { // Outlet - zero gradient (Neumann BC)
            // Copy from neighboring fluid node in the flow direction
            if (x > 0u) {
                let neighbor_idx = (x - 1u) + y * config.domain_size.x + z * config.domain_size.x * config.domain_size.y;
                // Only copy from fluid nodes
                if (lattice[neighbor_idx].node_type == 0u) {
                    for (var i = 0u; i < Q; i++) {
                        lattice[idx].f[i] = lattice[neighbor_idx].f[i];
                    }
                    lattice[idx].density = lattice[neighbor_idx].density;
                    lattice[idx].velocity = lattice[neighbor_idx].velocity;
                } else {
                    // If no valid neighbor, use outflow conditions
                    for (var i = 0u; i < Q; i++) {
                        lattice[idx].f[i] = equilibrium_distribution(i, 1.0, array<f32, 3>(0.1, 0.0, 0.0));
                    }
                    lattice[idx].density = 1.0;
                    lattice[idx].velocity = array<f32, 3>(0.1, 0.0, 0.0);
                }
            }
        }
        default: {} // Fluid and inlet nodes - no modification needed (handled in collision)
    }
}
