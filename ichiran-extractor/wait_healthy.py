#!/usr/bin/env python3
"""Poll the pooled-API /health endpoint until all workers are healthy.

Exits 0 when `healthy == pool_size`, 1 on timeout, 2 on argparse/usage error.
Polls every POLL_INTERVAL seconds, gives up after TIMEOUT seconds.
"""
import argparse
import json
import sys
import time
import urllib.error
import urllib.request

POLL_INTERVAL = 3
TIMEOUT = 300  # 5 minutes


def fetch_health(url: str, timeout: float = 4.0) -> dict | None:
    try:
        with urllib.request.urlopen(url, timeout=timeout) as r:
            return json.loads(r.read())
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, ConnectionError):
        return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://localhost:9100/health")
    ap.add_argument("--timeout", type=int, default=TIMEOUT)
    ap.add_argument("--interval", type=int, default=POLL_INTERVAL)
    args = ap.parse_args()

    deadline = time.monotonic() + args.timeout
    poll = 0
    while time.monotonic() < deadline:
        poll += 1
        h = fetch_health(args.url)
        if h is None:
            print(f"  poll {poll}: endpoint unreachable", flush=True)
        else:
            healthy = h.get("healthy", 0)
            pool_size = h.get("pool_size", 0)
            if pool_size > 0 and healthy == pool_size:
                print(f"  ALL-{pool_size}-HEALTHY (after {poll} polls)", flush=True)
                return 0
            print(f"  poll {poll}: healthy={healthy}/{pool_size}", flush=True)
        time.sleep(args.interval)

    print(f"  ERROR: not healthy after {args.timeout}s ({poll} polls)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
