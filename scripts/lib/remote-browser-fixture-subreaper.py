#!/usr/bin/env python3
"""Keep a Linux child-subreaper alive around the bounded Bash launcher."""

from __future__ import annotations

import ctypes
import os
import signal
import sys
import time


PR_SET_CHILD_SUBREAPER = 36
PR_SET_PDEATHSIG = 1


def process_token(pid: int) -> str | None:
    try:
        with open(f"/proc/{pid}/stat", encoding="ascii") as stat_file:
            process_tail = stat_file.read().rsplit(") ", 1)[1].split()
        token = process_tail[19]
    except (OSError, IndexError):
        return None
    return token if token.isdigit() else None


def process_parent_pid(pid: int) -> int | None:
    try:
        with open(f"/proc/{pid}/status", encoding="ascii") as status_file:
            return next(
                int(line.split()[1])
                for line in status_file
                if line.startswith("PPid:")
            )
    except (OSError, StopIteration, ValueError):
        return None


def observed_process_identity(pid: int) -> tuple[int, str] | None:
    token_before = process_token(pid)
    if token_before is None:
        return None
    parent_pid = process_parent_pid(pid)
    token_after = process_token(pid)
    if parent_pid is None or token_after != token_before:
        return None
    return parent_pid, token_before


def descendant_identities(root_pid: int) -> tuple[list[tuple[int, str]], bool]:
    children: dict[int, list[tuple[int, str]]] = {}
    scan_complete = True
    try:
        entries = list(os.scandir("/proc"))
    except OSError:
        return [], False
    for entry in entries:
        if not entry.name.isdigit():
            continue
        pid = int(entry.name)
        identity = observed_process_identity(pid)
        if identity is None:
            if os.path.exists(entry.path):
                scan_complete = False
            continue
        parent_pid, token = identity
        children.setdefault(parent_pid, []).append((pid, token))
    ordered: list[tuple[int, str]] = []

    def visit(parent_pid: int) -> None:
        for child_pid, token in children.get(parent_pid, []):
            visit(child_pid)
            ordered.append((child_pid, token))

    visit(root_pid)
    return ordered, scan_complete


def install_parent_death_cleanup(libc: ctypes.CDLL, expected_parent_pid: int) -> None:
    cleanup_active = False

    def reap_exited_children() -> None:
        while True:
            try:
                child_pid, _ = os.waitpid(-1, os.WNOHANG)
            except ChildProcessError:
                return
            except InterruptedError:
                continue
            if child_pid == 0:
                return

    def cleanup_descendants(_signal_number: int, _frame: object) -> None:
        nonlocal cleanup_active
        if cleanup_active:
            return
        cleanup_active = True
        while True:
            descendants, scan_complete = descendant_identities(os.getpid())
            if not descendants and scan_complete:
                os._exit(143)
            for descendant_pid, expected_token in descendants:
                if process_token(descendant_pid) != expected_token:
                    continue
                try:
                    os.kill(descendant_pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                except PermissionError:
                    continue
            time.sleep(0.05)
            reap_exited_children()

    signal.signal(signal.SIGUSR2, cleanup_descendants)
    if os.getppid() != expected_parent_pid:
        os.kill(os.getpid(), signal.SIGUSR2)
    if libc.prctl(PR_SET_PDEATHSIG, signal.SIGUSR2, 0, 0, 0) != 0:
        error_number = ctypes.get_errno()
        raise OSError(error_number, "could not establish bounded parent-death signal")
    if os.getppid() != expected_parent_pid:
        os.kill(os.getpid(), signal.SIGUSR2)


def wait_for_parent_admission() -> None:
    admission_path = os.environ.get("DEVE_REMOTE_FIXTURE_SUBREAPER_ADMISSION_PATH")
    if admission_path is None:
        return
    if not admission_path or "\x00" in admission_path:
        raise OSError("invalid subreaper admission path")
    while True:
        try:
            with open(admission_path, encoding="ascii") as admission_file:
                admission = admission_file.read()
        except FileNotFoundError:
            admission = ""
        if admission == "admitted\n":
            return
        if admission:
            raise OSError("invalid subreaper admission capability")
        time.sleep(0.01)


def main() -> int:
    if len(sys.argv) < 7 or sys.argv[5] != "--":
        print("remote-browser-fixture: bounded subreaper command is empty", file=sys.stderr)
        return 125
    completion_path = sys.argv[1]
    failure_path = sys.argv[2]
    launcher_identity_path = sys.argv[3]
    try:
        expected_parent_pid = int(sys.argv[4])
    except ValueError:
        expected_parent_pid = 0
    command = sys.argv[6:]
    if not command or any(
        "\x00" in value
        for value in (completion_path, failure_path, launcher_identity_path)
    ) or expected_parent_pid <= 1:
        print("remote-browser-fixture: bounded subreaper arguments are invalid", file=sys.stderr)
        return 125
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
        error_number = ctypes.get_errno()
        print(
            "remote-browser-fixture: could not establish bounded child-subreaper "
            f"capability; errno={error_number}",
            file=sys.stderr,
        )
        return 125
    try:
        install_parent_death_cleanup(libc, expected_parent_pid)
    except OSError as error:
        print(
            "remote-browser-fixture: could not establish bounded parent-death cleanup; "
            f"errno={error.errno}",
            file=sys.stderr,
        )
        return 125
    signal.signal(signal.SIGINT, signal.SIG_IGN)
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    try:
        wait_for_parent_admission()
    except OSError:
        print(
            "remote-browser-fixture: subreaper admission capability is invalid",
            file=sys.stderr,
        )
        return 125
    child_pid = os.fork()
    if child_pid == 0:
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        signal.signal(signal.SIGTERM, signal.SIG_DFL)
        os.execvp(command[0], command)
        os._exit(125)

    try:
        child_token = process_token(child_pid)
        if child_token is None:
            raise OSError("could not read bounded launcher process token")
        identity_tmp_path = f"{launcher_identity_path}.{os.getpid()}.tmp"
        descriptor = os.open(
            identity_tmp_path,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
            0o600,
        )
        with os.fdopen(descriptor, "w", encoding="ascii") as identity_file:
            identity_file.write(f"{child_pid}|{child_token}\n")
        os.replace(identity_tmp_path, launcher_identity_path)
    except (OSError, IndexError):
        os.kill(child_pid, signal.SIGKILL)
        os.waitpid(child_pid, 0)
        print(
            "remote-browser-fixture: could not publish bounded launcher identity",
            file=sys.stderr,
        )
        return 125

    _, wait_status = os.waitpid(child_pid, 0)
    child_status = os.waitstatus_to_exitcode(wait_status)
    if child_status < 0:
        child_status = 128 + (-child_status)
    try:
        with open(completion_path, encoding="ascii") as completion_file:
            completion = completion_file.read()
    except OSError:
        completion = ""
    if completion == "released\n":
        return child_status

    try:
        failure_tmp_path = f"{failure_path}.{os.getpid()}.tmp"
        descriptor = os.open(
            failure_tmp_path,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
            0o600,
        )
        with os.fdopen(descriptor, "w", encoding="ascii") as failure_file:
            failure_file.write(f"{child_status}\n")
        os.replace(failure_tmp_path, failure_path)
    except OSError as error:
        print(
            "remote-browser-fixture: could not publish bounded launcher failure; "
            f"errno={error.errno}",
            file=sys.stderr,
        )
    while True:
        signal.pause()


if __name__ == "__main__":
    raise SystemExit(main())
