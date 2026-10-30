#!/usr/bin/env python3
"""Copy the canonical passive bridge, or check byte equality without executing it."""
import argparse
from pathlib import Path
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check only; never rewrite resources")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    source = root / "web/code0-bridge.js"
    destination = root / "Sources/AIMeterApp/Resources/Code0/code0-bridge.js"
    data = source.read_bytes()
    if args.check:
        if not destination.is_file() or destination.read_bytes() != data:
            print("Code0 bridge resource differs from canonical source", file=sys.stderr)
            return 1
        print("Code0 bridge resource matches canonical source")
    else:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
