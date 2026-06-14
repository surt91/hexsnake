"""Run several independent training runs in parallel across all CPU cores.

For HexSnake the env and the net are tiny, so a *single* PPO/DQN run is
largely single-threaded: the learner dominates and env-level parallelism
(SubprocVecEnv) barely helps because each env step is nanoseconds of Rust.
The effective way to use all cores is therefore embarrassingly parallel —
launch many independent seeds at once and keep the best.

Each run is a separate OS process pinned to one thread (so N runs ≈ N cores).

Usage:
    uv run --extra train python parallel_train.py --algo ppo --runs 8 \
        --timesteps 1000000 --boundary mixed
    # then benchmark the produced *.mlp and embed the best (see the guides).
"""

import argparse
import concurrent.futures
import os
import subprocess
import sys


def run_one(algo: str, seed: int, timesteps: int, out_dir: str, extra: list[str]) -> str:
    out = os.path.join(out_dir, f"{algo}-seed{seed}.mlp")
    cmd = [
        sys.executable,
        f"train_{algo}.py",
        "--timesteps",
        str(timesteps),
        "--seed",
        str(seed),
        "--out",
        out,
        *extra,
    ]
    # One thread per run so the parallel processes don't oversubscribe.
    env = {**os.environ, "OMP_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"}
    subprocess.run(cmd, check=True, env=env, stdout=subprocess.DEVNULL)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--algo", choices=["ppo", "dqn"], required=True)
    ap.add_argument("--runs", type=int, default=os.cpu_count() or 4)
    ap.add_argument("--timesteps", type=int, default=1_000_000)
    ap.add_argument("--boundary", default="mixed", choices=["walls", "torus", "mixed"])
    ap.add_argument("--out-dir", default="training-out")
    # PPO runs use a single env each (n_envs=1) so the run count, not the env
    # count, fills the cores; passed through to train_ppo.py.
    args, passthrough = ap.parse_known_args()

    os.makedirs(args.out_dir, exist_ok=True)
    extra = ["--boundary", args.boundary, *passthrough]
    if args.algo == "ppo":
        extra += ["--n-envs", "1"]

    print(f"launching {args.runs} parallel {args.algo} runs ({args.timesteps} steps each)")
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.runs) as pool:
        futures = [
            pool.submit(run_one, args.algo, seed, args.timesteps, args.out_dir, extra)
            for seed in range(args.runs)
        ]
        for f in concurrent.futures.as_completed(futures):
            print("finished:", f.result())

    print(
        "\nDone. Benchmark the runs and embed the best, e.g.:\n"
        "  cp <best>.mlp ../crates/snake-core/assets/"
        f"{args.algo}/policy.mlp\n"
        "  cargo run --release -p snake-core --example benchmark 30 5000"
    )


if __name__ == "__main__":
    main()
