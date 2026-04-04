import pltrs
import math

# Generate scatter data as (x, y) pairs
data = [(i / 50.0, math.sin(i / 50.0 * 6.28) * 0.4 + 0.5) for i in range(50)]
annotations = [
    (0.25, math.sin(0.25 * 6.28) * 0.4 + 0.5, "rising"),
    (0.75, math.sin(0.75 * 6.28) * 0.4 + 0.5, "falling"),
]

fig = pltrs.Scatter(data, annotations=annotations)
fig.show()
