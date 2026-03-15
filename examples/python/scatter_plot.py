import pltrs
import math

# Generate scatter data as (x, y) pairs
data = [(i / 50.0, math.sin(i / 50.0 * 6.28) * 0.4 + 0.5) for i in range(50)]

fig = pltrs.Scatter(data)
fig.show()
