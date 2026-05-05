import argparse
import math
import os
import statistics
import subprocess
import sys
import time


def make_series():
    series_a = [(i / 10.0, math.sin(i / 10.0)) for i in range(100)]
    series_b = [(i / 10.0, math.cos(i / 10.0)) for i in range(100)]
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
    fig.show()


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

    start = None

    def close_soon():
        plt.close(fig)

    def on_draw(_event):
        nonlocal start
        if start is None:
            return
        elapsed_ms = (time.perf_counter() - start) * 1_000.0
        print(f"MATPLOTLIB_BENCHMARK_MS={elapsed_ms:.3f}", flush=True)
        fig.canvas.mpl_disconnect(draw_id)
        timer = fig.canvas.new_timer(interval=0)
        timer.add_callback(close_soon)
        timer.start()
        start = None

    draw_id = fig.canvas.mpl_connect("draw_event", on_draw)
    start = time.perf_counter()
    plt.show()


def parse_elapsed_ms(output, marker):
    prefix = f"{marker}="
    for line in output.splitlines():
        if line.startswith(prefix):
            return float(line[len(prefix) :])
    return None


def run_one(library):
    env = os.environ.copy()
    if library == "pltrs":
        env["PLTRS_BENCHMARK_ONESHOT"] = "1"

    result = subprocess.run(
        [sys.executable, __file__, "--child", library],
        cwd=os.path.dirname(os.path.dirname(os.path.dirname(__file__))),
        env=env,
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

    marker = "PLTRS_BENCHMARK_MS" if library == "pltrs" else "MATPLOTLIB_BENCHMARK_MS"
    elapsed_ms = parse_elapsed_ms(result.stdout, marker)
    if elapsed_ms is not None:
        return elapsed_ms

    if library == "pltrs":
        raise RuntimeError(
            "pltrs benchmark output was missing the first-frame timing marker.\n"
            "The most likely cause is that the Python extension was not rebuilt after the "
            "Rust backend benchmark hook was added.\n"
            "Rebuild it with:\n"
            "  maturin develop -m crates/pltrs_python/Cargo.toml\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )

    raise RuntimeError(
        f"{library} benchmark output missing {marker}\n"
        f"stdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )


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
