#!/usr/bin/env python3
"""Bounded Docker diagnostics and allowlisted full-stream evidence parser.

The producer writes a NUL-prefixed exit-status sentinel after its complete
output. This process always drains stdin before deciding whether to emit a
result, so callers never need to buffer Docker logs in a shell variable,
environment variable, or temporary raw file.
"""

from __future__ import annotations

from collections import deque
import re
import sys


DIAGNOSTIC_MAX_BYTES = 65_536
DIAGNOSTIC_MAX_LINES = 160
DIAGNOSTIC_MAX_LINE_BYTES = 8_192
STATUS_PREFIX = b"\x00DEVE_DOCKER_PRODUCER_STATUS:"
TOKEN_FRAME_PREFIX = b"\x00DEVE_DOCKER_TOKEN_FRAME_V1:"
TOKEN_STATUS = 3
PARSER_ERROR_STATUS = 4
NO_EVIDENCE_STATUS = 1
MAX_TOKEN_COUNT = 8
MAX_TOKEN_BYTES = 8_192

ANSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
BEARER_RE = re.compile(r"(?i)(authorization\s*:\s*bearer\s+)[^\s,;]+")
JSON_AUTHORIZATION_RE = re.compile(
    r'(?i)([\"]?authorization[\"]?\s*:\s*\")\s*Bearer\s+(?:\\.|[^\"\\])*(\")'
)
JSON_SECRET_RE = re.compile(
    r"(?i)([\"']?(?:(?:[a-z0-9]+_)*(?:password|passwd|pwd|token|secret|api[_ -]?key))[\"']?\s*:\s*\")"
    r"(?:\\.|[^\"\\])*"
    r"(\")"
)
KEY_VALUE_SECRET_RE = re.compile(
    r"(?i)((?:\b[a-z0-9]+_)*(?:password|passwd|pwd|token|secret|api[_ -]?key)\b\s*[=:]\s*)"
    r"(?:\"(?:\\.|[^\"\\])*\"|'[^']*'|[^\s,;]+)"
)


class BoundedLineReader:
    """Drain a byte stream while retaining at most one bounded line."""

    def __init__(self, stream, tokens: list[bytes], max_line_bytes: int):
        self.stream = stream
        self.tokens = tokens
        self.max_line_bytes = max_line_bytes
        self.pending = bytearray()
        self.pending_truncated = False
        self.line_started = False
        self.token_window = b""
        self.token_found = False
        max_token = max((len(token) for token in tokens), default=0)
        self.token_window_bytes = max(0, max_token - 1)

    def _append_segment(self, segment: bytes) -> None:
        self.line_started = True
        room = self.max_line_bytes - len(self.pending)
        if room > 0:
            self.pending.extend(segment[:room])
        if len(segment) > max(0, room):
            self.pending_truncated = True

    def _finish_line(self):
        line = bytes(self.pending)
        truncated = self.pending_truncated
        self.pending.clear()
        self.pending_truncated = False
        self.line_started = False
        return line, truncated

    def __iter__(self):
        while True:
            chunk = self.stream.read(4096)
            if not chunk:
                break

            if self.tokens:
                combined = self.token_window + chunk
                if any(token in combined for token in self.tokens):
                    self.token_found = True
                if self.token_window_bytes:
                    self.token_window = combined[-self.token_window_bytes :]

            start = 0
            while True:
                newline = chunk.find(b"\n", start)
                if newline < 0:
                    self._append_segment(chunk[start:])
                    break
                self._append_segment(chunk[start:newline])
                yield self._finish_line()
                start = newline + 1

        if self.line_started:
            yield self._finish_line()


def redact_line(line: str) -> str:
    line = ANSI_RE.sub("", line)
    line = JSON_AUTHORIZATION_RE.sub(r"\1Bearer <redacted>\2", line)
    line = BEARER_RE.sub(r"\1<redacted>", line)
    line = JSON_SECRET_RE.sub(r"\1<redacted>\2", line)
    return KEY_VALUE_SECRET_RE.sub(r"\1<redacted>", line)


def clean_line(raw: bytes) -> str:
    return ANSI_RE.sub("", raw.decode("utf-8", errors="replace")).rstrip("\r")


def add_bounded_line(
    lines: deque[bytes], total_bytes: int, line: str, payload_bytes: int, max_lines: int
) -> int:
    encoded = line.encode("utf-8", errors="replace")[:DIAGNOSTIC_MAX_LINE_BYTES]
    encoded += b"\n"
    # The line cap is smaller than the total payload budget, but keep this
    # guard so a future budget change cannot make the ring unbounded.
    if len(encoded) > payload_bytes:
        encoded = encoded[-payload_bytes:]
    lines.append(encoded)
    total_bytes += len(encoded)
    while len(lines) > max_lines or total_bytes > payload_bytes:
        total_bytes -= len(lines.popleft())
    return total_bytes


def producer_status_from_line(raw: bytes) -> int | None:
    if not raw.startswith(STATUS_PREFIX):
        return None
    value = raw[len(STATUS_PREFIX) :].strip()
    if not value.isdigit():
        return -1
    return int(value)


def required_args(mode: str, args: list[str], count: int) -> bool:
    return mode in {"diagnostic", "token-scan"} or len(args) == count


def parse_stream(mode: str, args: list[str], tokens: list[bytes], direct: bool) -> int:
    if mode == "mesh-count" and not required_args(mode, args, 2):
        return 2
    if mode == "server-peer-id" and not required_args(mode, args, 0):
        return 2
    if mode == "authenticated-peer" and not required_args(mode, args, 2):
        return 2
    if mode == "remote-ops" and not required_args(mode, args, 2):
        return 2
    if mode == "sequence-gap-fault" and not required_args(mode, args, 0):
        return 2
    if mode == "sequence-gap-rejection" and not required_args(mode, args, 0):
        return 2
    if mode not in {
        "diagnostic",
        "token-scan",
        "mesh-count",
        "server-peer-id",
        "authenticated-peer",
        "remote-ops",
        "sequence-gap-fault",
        "sequence-gap-rejection",
    }:
        return 2

    reader = BoundedLineReader(sys.stdin.buffer, tokens, DIAGNOSTIC_MAX_LINE_BYTES)
    producer_status: int | None = None
    diagnostic_lines: deque[bytes] = deque()
    diagnostic_total = 0
    marker = b"--- deve docker diagnostics: bounded tail bytes=65536 lines=160 ---\n"
    payload_bytes = DIAGNOSTIC_MAX_BYTES - len(marker)

    mesh_count = 0
    latest_server_peer: str | None = None
    latest_authenticated: str | None = None
    latest_bound: str | None = None
    evidence_found = False

    mesh_peer = re.escape(args[0]) if mode == "mesh-count" else ""
    mesh_repo = re.escape(args[1]) if mode == "mesh-count" else ""
    mesh_bound = (
        re.compile(r"Session bound to peer " + mesh_peer + r" and repo " + mesh_repo)
        if mode == "mesh-count"
        else None
    )
    mesh_authenticated = (
        f'authenticated_peer_id="{args[0]}"' if mode == "mesh-count" else ""
    )
    server_pattern = re.compile(r"Server PeerID: ([^ \t\r\n]+)")
    auth_pattern = re.compile(r"authenticated_peer_id=([^ \t\r\n]+)")
    bound_pattern = (
        re.compile(r"Session bound to peer ([^ \t\r\n]+) and repo " + re.escape(args[1]))
        if mode == "authenticated-peer"
        else None
    )
    remote_handled = (
        re.compile(
            r"Handled [1-9][0-9]* remote ops from "
            + re.escape(args[0])
            + r" for repo "
            + re.escape(args[1])
        )
        if mode == "remote-ops"
        else None
    )
    remote_authenticated = (
        f'authenticated_peer_id="{args[0]}"' if mode == "remote-ops" else ""
    )
    applied_pattern = re.compile(r"applied_pushes=([0-9]+)")
    sequence_rejection = re.compile(
        r"non-contiguous remote ops: expected seq [0-9]+, received [0-9]+"
    )

    try:
        for raw_line, truncated in reader:
            status = producer_status_from_line(raw_line)
            if status is not None:
                producer_status = status
                continue

            if mode == "diagnostic":
                diagnostic_total = add_bounded_line(
                    diagnostic_lines,
                    diagnostic_total,
                    redact_line(clean_line(raw_line)),
                    payload_bytes,
                    DIAGNOSTIC_MAX_LINES - 1,
                )
                continue

            if truncated:
                continue
            line = clean_line(raw_line)
            if mode == "token-scan":
                continue
            if mode == "mesh-count":
                if "P2P mesh connector handshake completed" in line and mesh_authenticated in line:
                    mesh_count += 1
                elif mesh_bound is not None and mesh_bound.search(line):
                    mesh_count += 1
            elif mode == "server-peer-id":
                match = server_pattern.search(line)
                if match:
                    candidate = match.group(1).strip('"')
                    if 0 < len(candidate) <= 256:
                        latest_server_peer = candidate
            elif mode == "authenticated-peer":
                if "P2P mesh connector handshake completed" in line and f"peer_label={args[0]}" in line:
                    match = auth_pattern.search(line)
                    if match:
                        candidate = match.group(1).strip('"')
                        if 0 < len(candidate) <= 256:
                            latest_authenticated = candidate
                if bound_pattern is not None:
                    match = bound_pattern.search(line)
                    if match:
                        candidate = match.group(1).strip('"')
                        if 0 < len(candidate) <= 256:
                            latest_bound = candidate
            elif mode == "remote-ops":
                if remote_handled is not None and remote_handled.search(line):
                    evidence_found = True
                elif "P2P mesh connector handshake completed" in line and remote_authenticated in line:
                    match = applied_pattern.search(line)
                    if match and int(match.group(1)) > 0:
                        evidence_found = True
            elif mode == "sequence-gap-fault":
                if "P2P test fault injected sequence_gap" in line:
                    evidence_found = True
            elif mode == "sequence-gap-rejection":
                if sequence_rejection.search(line):
                    evidence_found = True
    except (OSError, UnicodeError):
        return PARSER_ERROR_STATUS

    # A production parser must see the producer's final sentinel. A direct
    # parser invocation is permitted only for the isolated stdin regression.
    if not direct and producer_status is None:
        return PARSER_ERROR_STATUS
    if producer_status not in (None, 0):
        return PARSER_ERROR_STATUS
    if reader.token_found:
        return TOKEN_STATUS

    if mode == "diagnostic":
        sys.stdout.buffer.write(marker)
        for line in diagnostic_lines:
            sys.stdout.buffer.write(line)
        return 0
    if mode == "token-scan":
        return 0
    if mode == "mesh-count":
        print(mesh_count)
        return 0
    if mode == "server-peer-id":
        if latest_server_peer is None:
            return NO_EVIDENCE_STATUS
        print(latest_server_peer)
        return 0
    if mode == "authenticated-peer":
        candidate = latest_authenticated or latest_bound
        if candidate is None:
            return NO_EVIDENCE_STATUS
        print(candidate)
        return 0
    if mode in {"remote-ops", "sequence-gap-fault", "sequence-gap-rejection"}:
        return 0 if evidence_found else NO_EVIDENCE_STATUS
    return 2


def read_exact(stream, size: int) -> bytes | None:
    raw = bytearray()
    while len(raw) < size:
        chunk = stream.read(size - len(raw))
        if not chunk:
            return None
        raw.extend(chunk)
    return bytes(raw)


def load_tokens_from_frame(stream) -> list[bytes] | None:
    header = stream.readline(64)
    if not header.startswith(TOKEN_FRAME_PREFIX) or not header.endswith(b"\n"):
        return None
    raw_count = header[len(TOKEN_FRAME_PREFIX) : -1]
    if not raw_count.isdigit():
        return None
    count = int(raw_count)
    if count < 1 or count > MAX_TOKEN_COUNT:
        return None

    tokens: list[bytes] = []
    for _ in range(count):
        raw_size = stream.readline(16)
        if not raw_size.endswith(b"\n") or not raw_size[:-1].isdigit():
            return None
        size = int(raw_size[:-1])
        if size < 1 or size > MAX_TOKEN_BYTES:
            return None
        token = read_exact(stream, size)
        if token is None:
            return None
        tokens.append(token)
    return tokens


def main() -> int:
    if len(sys.argv) < 2:
        return 2
    mode = sys.argv[1]
    args: list[str] = []
    tokens: list[bytes] = []
    direct = False
    token_frame = False
    index = 2
    while index < len(sys.argv):
        value = sys.argv[index]
        if value == "--direct":
            direct = True
            index += 1
            continue
        if value == "--token-frame":
            token_frame = True
            index += 1
            continue
        args.append(value)
        index += 1
    if token_frame:
        loaded_tokens = load_tokens_from_frame(sys.stdin.buffer)
        if loaded_tokens is None:
            return PARSER_ERROR_STATUS
        tokens = loaded_tokens
    return parse_stream(mode, args, tokens, direct)


if __name__ == "__main__":
    raise SystemExit(main())
