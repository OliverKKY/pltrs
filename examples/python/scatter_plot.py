import math

import pltrs

# Generate scatter data as (x, y) pairs
data_a = [(i / 50.0, math.sin(i / 50.0 * 6.28) * 0.4 + 0.5) for i in range(50)]
data_b = [(i / 50.0, math.cos(i / 50.0 * 6.28) * 0.3 + 0.5) for i in range(50)]
annotations = [
    (0.25, math.sin(0.25 * 6.28) * 0.4 + 0.5, "rising"),
    (0.75, math.sin(0.75 * 6.28) * 0.4 + 0.5, "falling"),
]

fig = pltrs.Scatter(
    [data_a, data_b],
    color=[(0.82, 0.24, 0.18), (0.14, 0.42, 0.86)],
    marker=["circle", "square"],
    title="Bodový graf",
    x_label="x",
    y_label="y",
    annotations=annotations,
)

fig.show()
fig.save("scatter_plot.png")
