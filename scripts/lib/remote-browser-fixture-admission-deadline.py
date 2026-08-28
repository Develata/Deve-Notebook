#!/usr/bin/env python3
"""Claim the Unix startup admission decision at one monotonic deadline."""

from __future__ import annotations

import math
import ctypes
import os
import signal
import sys
import time


PR_SET_PDEATHSIG = 1


def bind_parent_death(expected_parent: int) -> bool:
    if expected_parent <= 1 or os.getppid() != expected_parent:
        return False
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(PR_SET_PDEATHSIG, signal.SIGTERM, 0, 0, 0) != 0:
        return False
    return os.getppid() == expected_parent


def create_empty_capability(path: str) -> int:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags, 0o600)
    except FileExistsError:
        return 1
    except OSError:
        return 2
    os.close(descriptor)
    return 0


def main() -> int:
    if len(sys.argv) != 5 or any("\x00" in value for value in sys.argv[2:]):
        return 2
    try:
        delay = float(sys.argv[1])
    except ValueError:
        return 2
    if not math.isfinite(delay) or delay <= 0:
        return 2
    try:
        expected_parent = int(sys.argv[4])
    except ValueError:
        return 2
    if not bind_parent_death(expected_parent):
        return 2
    time.sleep(delay)
    marker_status = create_empty_capability(sys.argv[3])
    if marker_status != 0:
        return 2
    return create_empty_capability(sys.argv[2])


if __name__ == "__main__":
    raise SystemExit(main())
