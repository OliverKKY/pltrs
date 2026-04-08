# pltrs

pltrs "Plotters" is a high-performance Python plotting library implemented in Rust, leveraging `wgpu` for rendering.

## About

This project aims to provide a plotting library that offers improved performance and potentially more aesthetically pleasing visualizations compared to existing Python libraries. It's being developed as part of my bachelor's thesis and could potentially evolve into an open-source project.

## Features

* GPU-backed rendering through Rust and `wgpu`
* Python API for line, scatter, and bar plots
* Multiple series in a single figure
* Automatic axis ranges, ticks, labels, titles, and optional grid lines
* Text annotations in data coordinates
* Interactive zoom with the mouse wheel and middle-button drag panning
* Native-window display and offscreen PNG export

## Installation

For now, build the Python extension locally with `maturin`.

```bash
git clone https://github.com/OliverKKY/pltrs.git
cd pltrs
```

## Build

Make sure you have Rust installed and a Python virtual environment activated.

1.  **Build the Rust library and create the Python extension module**. Ensure you have `maturin` installed (`pip install maturin`). Run the following command in the project's root directory:

    ```bash
    maturin develop -m crates/pltrs_python/Cargo.toml
    ```

    This command builds the Rust code and creates a Python wheel in your development environment, allowing you to import `pltrs` directly.

2.  **Run an example script.**

    ```bash
    python examples/python/line_plot.py
    ```

    Executing `line_plot.py` should open a native window with a rendered plot.

## Example

```python
import math
import pltrs

series_a = [(i / 20.0, math.sin(i / 20.0)) for i in range(160)]
series_b = [(i / 20.0, math.cos(i / 20.0)) for i in range(160)]

fig = pltrs.Line(
    [series_a, series_b],
    title="Sine and Cosine",
    x_label="x",
    y_label="value",
    grid=True,
)
fig.show()
```

## Interaction

In the native window:

* Scroll the mouse wheel over the plot area to zoom
* Hold the middle mouse button and drag to pan
* Press `R` or `Home` to reset the view
