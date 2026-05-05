import argparse
import math

import matplotlib.pyplot as plt


def compute_limits(values, padding=0.05):
    lo = min(values)
    hi = max(values)
    value_range = hi - lo
    if abs(value_range) < float.fromhex("0x1.0p-52"):
        return (lo - 1.0, hi + 1.0)

    pad = value_range * padding
    return (lo - pad, hi + pad)


def build_figure():
    # Match examples/python/line_plot.py as closely as possible.
    series_a = [(i / 10.0, math.sin(i / 10.0)) for i in range(100)]
    series_b = [(i / 10.0, math.cos(i / 10.0)) for i in range(100)]
    annotations = [
        (math.pi / 2.0, 1.0, "text"),
        (3.0 * math.pi / 2.0, -1.0, "trough"),
    ]

    all_x = [x for series in (series_a, series_b) for x, _ in series]
    all_y = [y for series in (series_a, series_b) for _, y in series]

    fig = plt.figure(figsize=(8, 6), dpi=100, facecolor="white")
    ax = fig.add_axes([0.1, 0.1, 0.8, 0.8], facecolor="white")

    ax.set_xlim(*compute_limits(all_x))
    ax.set_ylim(*compute_limits(all_y))
    ax.axis("off")

    ax.plot(
        [x for x, _ in series_a],
        [y for _, y in series_a],
        color=(0.1, 0.2, 0.8),
        linewidth=9.0,
        solid_capstyle="round",
    )
    ax.plot(
        [x for x, _ in series_b],
        [y for _, y in series_b],
        color=(0.85, 0.25, 0.2),
        linewidth=6.0,
        solid_capstyle="round",
    )

    for x, y, text in annotations:
        ax.text(x, y, text, color="black", fontsize=18)

    return fig


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--save",
        help="Save the rendered figure to a file instead of opening a window.",
    )
    args = parser.parse_args()

    fig = build_figure()
    if args.save:
        fig.savefig(args.save, facecolor=fig.get_facecolor())
    else:
        plt.show()


if __name__ == "__main__":
    main()
