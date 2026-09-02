#!/bin/sh

(
  sleep 0.35
  printf 'delayed-tail-output\n'
) &
exit 0
