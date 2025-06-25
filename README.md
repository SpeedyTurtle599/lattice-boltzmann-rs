# D3Q27 Lattice-Boltzmann Navier-Stokes Solver

A high-performance 3D Lattice-Boltzmann solver for incompressible Navier-Stokes equations using GPU acceleration with WGPU and WGSL shaders.

## Features

- **D3Q27 Lattice Model**: 27-velocity 3D lattice for accurate representation of fluid flow
- **GPU Acceleration**: WGPU-based compute shaders for high performance
- **STL Geometry Input**: Import complex geometries from CAD software
- **ParaView Compatible Output**: VTK format for professional visualization
- **Configurable Parameters**: JSON-based configuration for easy parameter adjustment
- **Boundary Conditions**: Support for inlet, outlet, and no-slip wall boundaries

## Usage

```bash
# Build the project
cargo build --release

# Run a simulation
./target/release/lattice-boltzmann-rs config.json geometry.stl
```

## Configuration File

The configuration file (JSON format) contains all simulation parameters:

```json
{
  "domain": {
    "nx": 100,           // Grid points in x-direction
    "ny": 50,            // Grid points in y-direction  
    "nz": 50,            // Grid points in z-direction
    "dx": 0.01,          // Grid spacing in x (m)
    "dy": 0.01,          // Grid spacing in y (m)
    "dz": 0.01           // Grid spacing in z (m)
  },
  "physics": {
    "reynolds_number": 100.0,         // Reynolds number
    "inlet_velocity": [0.1, 0.0, 0.0], // Inlet velocity vector (m/s)
    "density": 1.0,                   // Fluid density (kg/m³)
    "viscosity": null                 // Optional: explicit viscosity (m²/s)
  },
  "simulation": {
    "max_iterations": 10000,          // Maximum number of time steps
    "convergence_tolerance": 1e-6,    // Convergence criterion
    "tau": null                       // Optional: explicit relaxation time
  },
  "output": {
    "output_directory": "./output",   // Output directory
    "output_frequency": 100,          // Output every N iterations
    "output_format": "vtk"           // Output format (vtk)
  }
}
```

## STL Geometry

The solver accepts STL files containing the solid geometry. The STL file should:
- Be in ASCII or binary format
- Contain triangulated surfaces representing solid boundaries
- Be positioned within the computational domain defined in the config

## Output Files

The solver generates VTK files compatible with ParaView:

- `geometry.vtk`: Visualization of the computational domain and boundary conditions
- `output_XXXXXX.vtk`: Transient flow solution files

### Available Fields for Visualization

- **Velocity**: 3D velocity vector field (for streamlines)
- **VelocityMagnitude**: Scalar velocity magnitude
- **Density**: Fluid density field  
- **Pressure**: Pressure field (derived from density)
- **Vorticity**: 3D vorticity vector (for flow structure analysis)
- **NodeType**: Boundary condition visualization (0=fluid, 1=solid, 2=inlet, 3=outlet)

## Visualization in ParaView

1. Open ParaView
2. Load `geometry.vtk` to visualize the domain setup
3. Load `output_*.vtk` files as a time series
4. Create visualizations:
   - **Streamlines**: Use the Velocity vector field
   - **Pressure contours**: Use the Pressure scalar field
   - **Velocity magnitude**: Use VelocityMagnitude for speed visualization
   - **Vortex structures**: Use Vorticity magnitude or Q-criterion

## Implementation Details

### D3Q27 Lattice Model

The solver uses the 27-velocity 3D lattice model with:
- 1 rest particle (0,0,0)
- 6 face-connected neighbors (±1,0,0), (0,±1,0), (0,0,±1)
- 12 edge-connected neighbors
- 8 corner-connected neighbors

### GPU Acceleration

The simulation runs entirely on the GPU using WGSL compute shaders:
- **Collision shader**: BGK collision operator with equilibrium distributions
- **Streaming shader**: Particle streaming to neighboring nodes
- **Boundary shader**: Implementation of boundary conditions

### Boundary Conditions

- **Inlet**: Prescribed velocity using equilibrium distributions
- **Outlet**: Zero-gradient (Neumann) boundary condition
- **Solid walls**: Bounce-back boundary condition for no-slip walls
- **Fluid**: Standard LBM collision and streaming

## Requirements

- Rust 1.70 or later
- GPU with WGPU support (DirectX 12, Vulkan, Metal, or WebGL)
- ParaView for visualization (free download from kitware.com)

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## References

1. Chen, S., & Doolen, G. D. (1998). Lattice Boltzmann method for fluid flows. Annual review of fluid mechanics, 30(1), 329-364.
2. Krüger, T., et al. (2017). The lattice Boltzmann method: principles and practice. Springer.
3. Mohamad, A. A. (2011). Lattice Boltzmann method: fundamentals and engineering applications with computer codes. Springer.
