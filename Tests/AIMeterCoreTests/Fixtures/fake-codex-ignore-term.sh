#!/bin/sh

if [ "$1" = "app-server" ]; then
  exec /usr/bin/python3 -c 'import signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); signal.signal(signal.SIGHUP, signal.SIG_IGN); time.sleep(10)'
fi

exit 2
