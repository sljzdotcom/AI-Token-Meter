#!/bin/sh

exec /usr/bin/python3 -c '
import os, sys, time

def mark(phase):
    if len(sys.argv) > 1:
        try:
            with open(sys.argv[1], "a") as trace:
                trace.write(f"{phase}|{time.time():.9f}|{os.getpid()}\n")
        except OSError:
            pass  # Diagnostics must not change the fixture exit behavior.

mark("python_started")
print("parent-exited", flush=True)
mark("parent_output_flushed")
pid = os.fork()
if pid == 0:
    mark("child_started")
    os.setsid()
    mark("child_detached")
    time.sleep(6)
else:
    mark("parent_exit_requested")
os._exit(0)
' "$@"
