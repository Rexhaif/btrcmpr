"""Linux directory locks for benchmark runs."""

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
    # Checkouts share the cache inode. The child holds the lock until it exits,
    # even if the scheduler dies.
    timing_lock = take_directory(
        cache, "another benchmark or profiler holds the timing lock"
    )
    try:
        child = subprocess.Popen(argv, pass_fds=(series_lock, timing_lock), **options)
    finally:
        os.close(timing_lock)
    with child:
        status = child.wait()
    if status:
        raise subprocess.CalledProcessError(status, argv)
