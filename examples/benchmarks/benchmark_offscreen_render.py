import argparse
import math
import os
import statistics
import subprocess
import sys
import tempfile
import time


def make_series(count=100):
    series_a = [(i / 10.0, math.sin(i / 10.0)) for i in range(count)]
    series_b = [(i / 10.0, math.cos(i / 10.0)) for i in range(count)]
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


def run_pltrs_child():
    import pltrs

    series_a, series_b, annotations = make_series()
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
        print(f"PLTRS_OFFSCREEN_MS={elapsed_ms:.3f}")
    finally:
        if os.path.exists(path):
            os.remove(path)


def run_matplotlib_child():
    import matplotlib.pyplot as plt

    series_a, series_b, annotations = make_series()
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
        print(f"MATPLOTLIB_OFFSCREEN_MS={elapsed_ms:.3f}")
    finally:
        plt.close(fig)
        if os.path.exists(path):
            os.remove(path)


def parse_elapsed_ms(output, marker):
    prefix = f"{marker}="
    for line in output.splitlines():
        if line.startswith(prefix):
            return float(line[len(prefix) :])
    raise RuntimeError(
        f"benchmark output missing {marker}\nstdout:\n{output}"
    )


def run_one(library):
    result = subprocess.run(
        [sys.executable, __file__, "--child", library],
        cwd=os.path.dirname(os.path.dirname(os.path.dirname(__file__))),
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{library} benchmark failed with code {result.returncode}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )

    marker = "PLTRS_OFFSCREEN_MS" if library == "pltrs" else "MATPLOTLIB_OFFSCREEN_MS"
    return parse_elapsed_ms(result.stdout, marker)


def summarize(name, samples):
    mean = statistics.fmean(samples)
    min_ms = min(samples)
    max_ms = max(samples)
    stddev = statistics.pstdev(samples) if len(samples) > 1 else 0.0
    print(
        f"{name}: mean={mean:.3f} ms min={min_ms:.3f} ms "
        f"max={max_ms:.3f} ms stddev={stddev:.3f} ms"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument(
        "--library",
        choices=["pltrs", "matplotlib", "both"],
        default="both",
    )
    parser.add_argument("--child", choices=["pltrs", "matplotlib"])
    args = parser.parse_args()

    if args.child == "pltrs":
        run_pltrs_child()
        return
    if args.child == "matplotlib":
        run_matplotlib_child()
        return

    libraries = ["pltrs", "matplotlib"] if args.library == "both" else [args.library]
    for library in libraries:
        samples = [run_one(library) for _ in range(args.runs)]
        summarize(library, samples)


if __name__ == "__main__":
    main()
