# Micro-Macro

A Rust-based interactive visualization tool for exploring Markov processes and macroscopic observables over them, as well as the induced dynamics.

## Overview

Micro-Macro provides a graphical interface to analyze relationships between microscopic state dynamics (Markov chains) and macroscopic observables. The tool allows you to define state transition systems, specify observables, and visualize the resulting observed dynamics.

## Features

- **State Graph Editor**: Define states and transition probabilities for your dynamical system
- **Observable Graph Editor**: Specify source states and destination observables with observation probabilities
- **Observed Graph Visualization**: Automatically compute and visualize the induced dynamics on observables
- **Multiple Layout Options**: Choose between circular and bipartite graph layouts
- **Interactive UI**: Built with egui for responsive graph manipulation and visualization
- **Heatmap Analysis**: View probability distributions and transition matrices
- **Save/Load Projects**: Serialize and restore your work

## Workspace Structure

The project is organized as a Cargo workspace with two crates:

- `crates/markov`: Core library implementing sparse row-stochastic Markov kernels
- `crates/micro-macro`: GUI application with graph editing and visualization

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```
