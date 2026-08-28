#!/bin/sh

if [ "$1" = "app-server" ]; then
  trap '' TERM HUP
  if [ -n "${AI_METER_TEST_PID_FILE:-}" ]; then
    printf '%s\n' "$$" > "$AI_METER_TEST_PID_FILE"
  fi
  exec /usr/bin/python3 -c 'import time; time.sleep(10)'
fi

exit 2
