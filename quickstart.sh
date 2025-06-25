#!/bin/bash

# Quick start script for Lattice-Boltzmann solver

echo "=== Lattice-Boltzmann Solver Quick Start ==="
echo

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Build the project
echo "Building the project..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "Error: Build failed"
    exit 1
fi

# Generate example STL file
echo "Generating example STL file..."
cargo run --example generate_example_stl
if [ $? -ne 0 ]; then
    echo "Error: Failed to generate example STL"
    exit 1
fi

# Create output directory
echo "Creating output directory..."
mkdir -p output

# Run the simulation
echo "Running example simulation..."
echo "This may take a few minutes depending on your GPU..."
RUST_LOG=info ./target/release/lattice-boltzmann-rs example_config.json example_cylinder.stl

if [ $? -eq 0 ]; then
    echo
    echo "=== Simulation completed successfully! ==="
    echo
    echo "Output files have been written to the './output' directory:"
    echo "  - geometry.vtk: Domain and boundary visualization"
    echo "  - output_*.vtk: Transient flow solution files"
    echo
    echo "To visualize the results:"
    echo "  1. Download and install ParaView (free): https://www.paraview.org/download/"
    echo "  2. Open ParaView"
    echo "  3. Load geometry.vtk to see the domain setup"
    echo "  4. Load output_*.vtk files as a time series to see the flow evolution"
    echo "  5. Try creating streamlines with the Velocity field"
    echo "  6. Visualize pressure contours with the Pressure field"
    echo "  7. Show vorticity magnitude to see flow structures"
    echo
    echo "Configuration file: example_config.json"
    echo "STL geometry file: example_cylinder.stl"
    echo
    echo "Modify these files to run your own simulations!"
else
    echo "Error: Simulation failed"
    exit 1
fi
