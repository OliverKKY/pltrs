# pltrs

pltrs "Plotters" is a high-performance Python plotting library implemented in Rust, leveraging `wgpu` for rendering.

## About

This project aims to provide a plotting library that offers improved performance and potentially more aesthetically pleasing visualizations compared to existing Python libraries. It's being developed as part of my bachelor's thesis and could potentially evolve into an open-source project.

## Features (Planned/In Progress)

*   Leverages Rust and `wgpu` for accelerated rendering.
*   (Add a couple more key features you plan to implement, e.g., "Intuitive API for plot creation," "Support for various plot types like line, scatter, and bar charts," etc.

## Installation

**(Note: Installation instructions will be added once the package is ready for distribution, e.g., on PyPI.)**

For now, you can explore the codebase by cloning the repository.

```bash
git clone https://github.com/OliverKKY/pltrs.git
cd pltrs
```

## Build

To explore the basic functionality and witness the window creation, follow these steps. But first make sure you have **Rust installed** and **virtual environment activated**.

1.  **Build the Rust library and create the Python extension module** using `maturin --version`. Ensure you have `maturin` installed (`pip install maturin`). Run the following command in the project's root directory:

    ```bash
    maturin develop -m crates/pltrs_python/Cargo.toml
    ```

    This command builds the Rust code and creates a Python wheel in your development environment, allowing you to import `pltrs` directly.

2.  **Run the example Python script.** Navigate to the directory containing your `main.py` (if it's separate from the root) and execute the script:

    ```bash
    python examples/python/line_plot.py
    ```

    Executing `line_plot.py` will call the `pltrs.show()` function, which should open a window displaying a green screen.
