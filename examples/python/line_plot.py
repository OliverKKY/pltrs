import math

import pltrs

# Generate sine wave data as (x, y) pairs
data = [(i / 10.0, math.sin(i / 10.0)) for i in range(100)]
annotations = [
    (math.pi / 2.0, 1.0, "text"),
    (3.0 * math.pi / 2.0, -1.0, "trough"),
]

fig = pltrs.Line(data, annotations=annotations)
fig.show()
