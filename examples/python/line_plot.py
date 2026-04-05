import math

import pltrs

# Generate two line series as (x, y) pairs
series_a = [(i / 10.0, math.sin(i / 10.0)) for i in range(100)]
series_b = [(i / 10.0, math.cos(i / 10.0)) for i in range(100)]
annotations = [
    (math.pi / 2.0, 1.0, "text"),
    (3.0 * math.pi / 2.0, -1.0, "trough"),
]

fig = pltrs.Line(
    [series_a, series_b],
    color=[(0.1, 0.2, 0.8), (0.85, 0.25, 0.2)],
    width=[9.0, 6.0],
    annotations=annotations,
)
fig.show()
