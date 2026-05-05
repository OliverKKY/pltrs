import pltrs

months = [6, 5, 4, 3, 2, 1]
stress = [72.0, 76.0, 81.0, 88.0, 124.0, 152.0]
annotations = [
    (3.0, 81.0, "already bad"),
    (4.0, 124.0, "off the chart"),
    (5.0, 152.0, "send help"),
]

fig = pltrs.Bar(
    [stress],
    x=(0.5, 5.5),
    y=(0.0, 150.0),
    color=(0.86, 0.22, 0.18),
    width=0.72,
    annotations=annotations,
    title="Stress Over Time Before My Finals",
    x_label="months before finals",
    y_label="stress level",
    grid=True,
)

fig.show()
fig.save("~/Pictures/finals_stress_bar_plot.png")
