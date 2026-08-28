#!/bin/sh

trap '' TERM HUP
/usr/bin/python3 -c 'import os, signal, time; os.setsid(); signal.signal(signal.SIGHUP, signal.SIG_IGN); signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(3)' &
wait
