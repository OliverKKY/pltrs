import pltrs

sales_2024 = [12.0, 18.0, 11.0, 21.0]
sales_2025 = [15.0, 16.0, 14.0, 24.0]
annotations = [
    (3.15, 24.0, "peak"),
]

fig = pltrs.Bar(
    [sales_2024, sales_2025],
    color=[(0.2, 0.45, 0.86), (0.92, 0.46, 0.2)],
    title="Quarterly Sales",
    x_label="quarter",
    y_label="revenue",
    # annotations=annotations,
    grid=True,
)
fig.show()
fig.save("~/Pictures/bar_plot.png")
