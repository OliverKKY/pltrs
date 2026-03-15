import math

import pltrs

# Generate sine wave data as (x, y) pairs
data = [(i / 10.0, math.sin(i / 10.0)) for i in range(100)]

fig = pltrs.Line(data)
fig.show()
