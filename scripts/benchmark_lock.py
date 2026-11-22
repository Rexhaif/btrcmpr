"""Advisory locks for a trial directory and active timed child on Linux."""

import fcntl
import os
import subprocess


def take_directory(path, busy):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        os.close(fd)
        raise SystemExit(busy) from None
    except BaseException:
        os.close(fd)
        raise
    return fd


def run_trial(cache, series_lock, argv, **options):
    # Frozen workspaces symlink the same binary cache. Lock its inode, not a
    # per-checkout pathname. Hand the timing lock to the measured child so it
    # is released on exit, even if the scheduling parent is stopped or killed.
    timing_lock = take_directory(
        cache, "another benchmark or profile is using the shared codec timing slot"
    )
    try:
        child = subprocess.Popen(argv, pass_fds=(series_lock, timing_lock), **options)
    finally:
        os.close(timing_lock)
    with child:
        status = child.wait()
    if status:
        raise subprocess.CalledProcessError(status, argv)
