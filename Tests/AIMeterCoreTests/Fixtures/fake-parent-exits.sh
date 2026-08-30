#!/bin/sh

exec /usr/bin/python3 -c 'import os, sys, time; print("parent-exited", flush=True); pid=os.fork(); (os.setsid(), time.sleep(6), os._exit(0)) if pid == 0 else os._exit(0)'
