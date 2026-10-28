#!/usr/bin/env python3
"""Summarize ram-explorer CSV outputs; optionally create PNG plots.

No dependencies for the printed report. matplotlib is optional for charts.
"""
import argparse
import csv
from collections import defaultdict
from pathlib import Path


def read_rows(path):
    with path.open(encoding="utf-8", newline="") as fp:
        return list(csv.DictReader(fp))


def main():
    parser = argparse.ArgumentParser(description="Analyze Repeat After Me Game sweep")
    parser.add_argument("--input-dir", default="results")
    parser.add_argument("--output-dir", default="reports")
    parser.add_argument("--no-plots", action="store_true")
    args = parser.parse_args()
    root = Path(args.input_dir)
    rows = read_rows(root / "summary.csv")
    rounds = read_rows(root / "rounds.csv")
    examples = read_rows(root / "examples.csv")
    if not rows:
        raise SystemExit("summary.csv has no data")
    print("Length | mode       | cases        | done %   | cycle %  | capped % | mean rounds | max length")
    print("-" * 105)
    for r in rows:
        capped = (int(r["token_limit"]) + int(r["round_limit"])) / int(r["cases"]) * 100
        print(f'{int(r["length"]):>6} | {r["mode"]:<10} | {int(r["cases"]):>12,} | '
              f'{float(r["done_rate"])*100:>7.2f} | {float(r["cycle_rate"])*100:>7.2f} | '
              f'{capped:>7.2f} | {float(r["mean_rounds"]):>11.2f} | {int(r["max_tokens_observed"]):>10,}')
    print("\nHighlighted examples:")
    for row in examples:
        if row["label"] in ("done", "cycle", "largest"):
            print(f'  n={row["length"]:<2} {row["label"]:<7} rounds={row["rounds"]:<3} '
                  f'peak={row["max_tokens_seen"]:<6} input={row["input"]}')

    if args.no_plots:
        return
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("\nmatplotlib is not installed; skipped PNG charts. Install with: pip install matplotlib")
        return
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    xs = [int(r["length"]) for r in rows]
    statuses = ["done", "cycle", "token_limit", "round_limit"]
    labels = {"done": "Terminated", "cycle": "Proven cycle", "token_limit": "Token cap", "round_limit": "Round cap"}
    fig, ax = plt.subplots(figsize=(9, 5))
    bottom = [0.] * len(xs)
    for status in statuses:
        ys = [int(r[status]) / int(r["cases"]) * 100 for r in rows]
        ax.bar(xs, ys, bottom=bottom, label=labels[status])
        bottom = [a + b for a, b in zip(bottom, ys)]
    ax.set_xlabel("Initial token length")
    ax.set_ylabel("Percentage of tested inputs")
    ax.set_title("Repeat After Me Game: outcomes")
    ax.set_ylim(0, 100)
    ax.legend()
    fig.tight_layout()
    fig.savefig(out / "outcomes.png", dpi=160)
    plt.close(fig)

    by_length = defaultdict(dict)
    for row in rounds:
        if row["status"] == "done":
            n = int(row["length"])
            r = int(row["rounds"])
            by_length[n][r] = int(row["cases"])
    fig, ax = plt.subplots(figsize=(9, 5))
    for n in xs:
        points = sorted(by_length[n].items())
        if points:
            ax.plot([p[0] for p in points], [p[1] for p in points], marker="o", label=f"n={n}")
    ax.set_xlabel("Rounds to termination")
    ax.set_ylabel("Count")
    ax.set_title("Round distribution (normally terminated inputs)")
    ax.legend(fontsize="small", ncol=2)
    fig.tight_layout()
    fig.savefig(out / "termination_rounds.png", dpi=160)
    plt.close(fig)
    print(f"\nCharts written to {out}/outcomes.png and {out}/termination_rounds.png")
    print("Note: limits are inconclusive; sampled rows are estimates, not exhaustive probabilities.")


if __name__ == "__main__":
    main()
