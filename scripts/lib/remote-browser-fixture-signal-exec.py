#!/usr/bin/env python3
"""Reset async-shell signal dispositions before execing a fixture worker."""

from __future__ import annotations

import os
import signal
import sys


def main() -> int:
    command = sys.argv[1:]
    if not command or any("\x00" in argument for argument in command):
        return 125

    signal.signal(signal.SIGINT, signal.SIG_DFL)
    signal.signal(signal.SIGTERM, signal.SIG_DFL)
    try:
        os.execvp(command[0], command)
    except OSError:
        return 125


if __name__ == "__main__":
    raise SystemExit(main())
