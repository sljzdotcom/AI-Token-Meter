#!/bin/sh

terminal_path="$(tty)"
case "$terminal_path" in
  /dev/ttys*) ;;
  *) exit 2 ;;
esac

exec 0<&- 1>&- 2>&-
sleep 0.05
exec 1>"$terminal_path" 2>&1
printf 'reopened-tail-output\n'
