import math

import matplotlib.pyplot as plt


series_a = [(i / 10.0, math.sin(i / 10.0)) for i in range(100)]
series_b = [(i / 10.0, math.cos(i / 10.0)) for i in range(100)]

plt.plot(
    [x for x, _ in series_a],
    [y for _, y in series_a],
    color=(0.1, 0.2, 0.8),
    linewidth=9.0,
)
plt.plot(
    [x for x, _ in series_b],
    [y for _, y in series_b],
    color=(0.85, 0.25, 0.2),
    linewidth=6.0,
)
plt.text(math.pi / 2.0, 1.0, "text")
plt.text(3.0 * math.pi / 2.0, -1.0, "trough")
plt.show()
