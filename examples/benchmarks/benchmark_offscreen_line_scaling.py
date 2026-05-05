import argparse
import math
import os
import subprocess
import sys
import tempfile
import time


def make_series(count):
    step = 10.0 / max(count - 1, 1)
    series_a = [(i * step, math.sin(i * step)) for i in range(count)]
    series_b = [(i * step, math.cos(i * step)) for i in range(count)]
    annotations = [
        (math.pi / 2.0, 1.0, "text"),
        (3.0 * math.pi / 2.0, -1.0, "trough"),
    ]
    return series_a, series_b, annotations


def compute_limits(values, padding=0.05):
    lo = min(values)
    hi = max(values)
    value_range = hi - lo
    if abs(value_range) < float.fromhex("0x1.0p-52"):
        return (lo - 1.0, hi + 1.0)

    pad = value_range * padding
    return (lo - pad, hi + pad)


def run_pltrs_child(points):
    import pltrs

    series_a, series_b, annotations = make_series(points)
    fig = pltrs.Line(
        [series_a, series_b],
        color=[(0.1, 0.2, 0.8), (0.85, 0.25, 0.2)],
        width=[9.0, 6.0],
        annotations=annotations,
    )

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        path = tmp.name
    try:
        start = time.perf_counter()
        fig.save(path)
        elapsed_ms = (time.perf_counter() - start) * 1_000.0
        print(f"PLTRS_SCALING_MS={elapsed_ms:.3f}")
    finally:
        if os.path.exists(path):
            os.remove(path)


def run_matplotlib_child(points):
    import matplotlib.pyplot as plt

    series_a, series_b, annotations = make_series(points)
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

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        path = tmp.name
    try:
        start = time.perf_counter()
        fig.savefig(path, facecolor=fig.get_facecolor())
        elapsed_ms = (time.perf_counter() - start) * 1_000.0
        print(f"MATPLOTLIB_SCALING_MS={elapsed_ms:.3f}")
    finally:
        plt.close(fig)
        if os.path.exists(path):
            os.remove(path)


def parse_elapsed_ms(output, marker):
    prefix = f"{marker}="
    for line in output.splitlines():
        if line.startswith(prefix):
            return float(line[len(prefix) :])
    raise RuntimeError(f"benchmark output missing {marker}\nstdout:\n{output}")


def run_one(library, points):
    result = subprocess.run(
        [sys.executable, __file__, "--child", library, "--points", str(points)],
        cwd=os.path.dirname(os.path.dirname(os.path.dirname(__file__))),
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{library} benchmark failed for {points} points with code {result.returncode}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )

    marker = "PLTRS_SCALING_MS" if library == "pltrs" else "MATPLOTLIB_SCALING_MS"
    return parse_elapsed_ms(result.stdout, marker)


def parse_sizes(raw):
    return [int(part.strip()) for part in raw.split(",") if part.strip()]


def summarize_row(points, pltrs_ms=None, matplotlib_ms=None):
    pltrs_text = f"{pltrs_ms:.3f}" if pltrs_ms is not None else "-"
    matplotlib_text = f"{matplotlib_ms:.3f}" if matplotlib_ms is not None else "-"
    ratio_text = "-"
    if pltrs_ms is not None and matplotlib_ms is not None and matplotlib_ms > 0:
        ratio_text = f"{pltrs_ms / matplotlib_ms:.2f}x"
    print(
        f"{points:>8} points | pltrs {pltrs_text:>10} ms | "
        f"matplotlib {matplotlib_text:>10} ms | ratio {ratio_text:>8}"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument(
        "--library",
        choices=["pltrs", "matplotlib", "both"],
        default="both",
    )
    parser.add_argument("--sizes", default="100,1000,10000,100000")
    parser.add_argument("--points", type=int, default=100)
    parser.add_argument("--child", choices=["pltrs", "matplotlib"])
    args = parser.parse_args()

    if args.child == "pltrs":
        run_pltrs_child(args.points)
        return
    if args.child == "matplotlib":
        run_matplotlib_child(args.points)
        return

    sizes = parse_sizes(args.sizes)
    print("offscreen line scaling benchmark")
    for points in sizes:
        pltrs_ms = None
        matplotlib_ms = None
        if args.library in ("pltrs", "both"):
            pltrs_samples = [run_one("pltrs", points) for _ in range(args.runs)]
            pltrs_ms = sum(pltrs_samples) / len(pltrs_samples)
        if args.library in ("matplotlib", "both"):
            matplotlib_samples = [
                run_one("matplotlib", points) for _ in range(args.runs)
            ]
            matplotlib_ms = sum(matplotlib_samples) / len(matplotlib_samples)
        summarize_row(points, pltrs_ms, matplotlib_ms)


if __name__ == "__main__":
    main()
