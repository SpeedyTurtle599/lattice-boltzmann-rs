# Technical Documentation

## Architecture Overview

The Lattice-Boltzmann solver is built with a modular architecture:

```
src/
├── lib.rs          # Module exports and type definitions
├── main.rs         # Command-line interface and main execution
├── config.rs       # Configuration parsing and validation
├── geometry.rs     # STL loading and geometry processing
├── lattice.rs      # D3Q27 lattice model implementation
├── gpu.rs          # WGPU context and GPU management
├── solver.rs       # Main simulation loop
├── output.rs       # VTK file writing
└── shaders/        # WGSL compute shaders
    ├── collision.wgsl
    ├── streaming.wgsl
    └── boundary.wgsl
```

## D3Q27 Lattice Model

### Velocity Set

The D3Q27 model uses 27 discrete velocities in 3D space:
- 1 rest particle: (0,0,0)
- 6 face neighbors: (±1,0,0), (0,±1,0), (0,0,±1)
- 12 edge neighbors: (±1,±1,0), (±1,0,±1), (0,±1,±1)  
- 8 corner neighbors: (±1,±1,±1)

### Weights

Each velocity direction has an associated weight for equilibrium calculations:
- w₀ = 8/27 (rest particle)
- w₁₋₆ = 2/27 (face neighbors)
- w₇₋₁₈ = 1/54 (edge neighbors)
- w₁₉₋₂₆ = 1/216 (corner neighbors)

### Equilibrium Distribution

The equilibrium distribution function is:

```
f_i^eq = w_i * ρ * (1 + (c_i·u)/c_s² + (c_i·u)²/(2c_s⁴) - u²/(2c_s²))
```

Where:
- w_i: weight for direction i
- ρ: density
- c_i: lattice velocity vector
- u: macroscopic velocity
- c_s² = 1/3: speed of sound squared

## GPU Implementation

### Compute Shaders

The simulation uses three main compute shaders:

1. **Collision Shader** (`collision.wgsl`):
   - Calculates macroscopic quantities (density, velocity)
   - Applies BGK collision operator
   - Updates distribution functions

2. **Streaming Shader** (`streaming.wgsl`):
   - Propagates distribution functions to neighboring nodes
   - Handles periodic/bounce-back boundaries at domain edges

3. **Boundary Shader** (`boundary.wgsl`):
   - Applies boundary conditions:
     - Solid walls: bounce-back
     - Inlet: prescribed velocity
     - Outlet: zero gradient

### Memory Layout

The lattice data is stored as a structure of arrays (SoA) format for optimal GPU memory access:

```rust
struct LatticePoint {
    f: [f32; 27],        // Distribution functions
    density: f32,        // Macroscopic density
    velocity: [f32; 3],  // Macroscopic velocity
    node_type: u32,      // Boundary condition type
    _padding: [u32; 3],  // Alignment padding
}
```

### Workgroup Organization

- Workgroup size: 8×8×1 threads
- Each thread processes one lattice node
- Workgroups are dispatched to cover the entire 3D domain

## Boundary Conditions

### Node Types

- **Type 0 (Fluid)**: Standard LBM collision and streaming
- **Type 1 (Solid)**: Bounce-back boundary condition
- **Type 2 (Inlet)**: Prescribed velocity using equilibrium distributions
- **Type 3 (Outlet)**: Zero-gradient (Neumann) boundary condition

### Implementation Details

#### Bounce-back (Solid walls)
For solid nodes, the distribution function is reflected:
```
f_i(x_wall, t+1) = f_ī(x_wall, t)
```
Where ī is the opposite direction of i.

#### Inlet (Prescribed velocity)
Inlet nodes are set to equilibrium distributions with prescribed velocity:
```
f_i = f_i^eq(ρ_inlet, u_inlet)
```

#### Outlet (Zero gradient)
Outlet nodes copy values from upstream fluid nodes to maintain zero gradient.

## Physical Units and Scaling

### Lattice Units vs Physical Units

The solver uses lattice units internally and converts to physical units for output.

**Length scaling:**
- Δx = physical_length / lattice_length
- Grid spacing in config file sets physical dimensions

**Time scaling:**
- Δt = (Δx)² / (ν * (τ - 0.5) * 2)
- Where τ is the relaxation time

**Velocity scaling:**
- u_phys = u_lattice * Δx / Δt

### Reynolds Number

The Reynolds number is maintained through the relation:
```
Re = U * L / ν
```

Where:
- U: characteristic velocity
- L: characteristic length  
- ν: kinematic viscosity

The relaxation time τ is calculated as:
```
τ = 3ν/c_s² + 0.5 = 3ν + 0.5
```

## Performance Considerations

### Memory Access Patterns

The GPU implementation is optimized for:
- Coalesced memory access in compute shaders
- Minimal branching in shader code
- Efficient use of workgroup shared memory

### Scalability

Performance scales with:
- GPU compute units and memory bandwidth
- Domain size (O(N³) for N³ lattice)
- Number of iterations

Typical performance on modern GPUs:
- RTX 4090: ~500 MLUPS (Million Lattice Updates Per Second)
- RTX 3080: ~300 MLUPS
- M1 Max: ~150 MLUPS

### Memory Requirements

For a domain of size N³:
- Each lattice point: 32 floats + 4 integers = 144 bytes
- Total memory: N³ × 144 bytes
- Example: 100³ domain = ~1.4 GB GPU memory

## Validation and Verification

### Test Cases

Common validation cases for LBM solvers:

1. **Poiseuille Flow**: Analytical solution available
2. **Couette Flow**: Simple shear flow validation
3. **Flow Around Cylinder**: Standard CFD benchmark
4. **Lid-Driven Cavity**: Well-documented reference solutions

### Convergence Criteria

The solver monitors convergence through:
- Maximum velocity change between iterations
- Residual norms of macroscopic quantities
- Mass conservation check

## Extensions and Modifications

### Adding New Boundary Conditions

1. Define new node type in `lattice.rs`
2. Add handling in `boundary.wgsl` shader
3. Update geometry processing in `geometry.rs`

### Multi-Phase Flows

The current implementation can be extended for:
- Shan-Chen multiphase model
- Free energy models
- Phase field methods

### Turbulence Modeling

Large Eddy Simulation (LES) can be added through:
- Smagorinsky subgrid-scale model
- Dynamic LES models
- Wall-adapting local eddy-viscosity (WALE)

## Troubleshooting

### Common Issues

1. **GPU Memory Errors**: Reduce domain size or increase system memory
2. **Slow Convergence**: Check Reynolds number and relaxation time
3. **Instability**: Reduce time step (increase τ) or check boundary conditions
4. **Incorrect Results**: Verify STL geometry and boundary placement

### Debug Output

Enable debug logging with:
```bash
RUST_LOG=debug ./target/release/lattice-boltzmann-rs config.json geometry.stl
```

### Performance Profiling

Use tools like:
- `nvidia-smi` for GPU utilization
- `cargo flamegraph` for CPU profiling
- WGPU debug layers for GPU debugging
