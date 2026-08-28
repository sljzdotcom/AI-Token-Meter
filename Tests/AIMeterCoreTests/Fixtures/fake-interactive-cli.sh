#!/bin/sh

if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"loggedIn":true,"authMethod":"oauth","apiProvider":"firstParty"}\n'
  exit 0
fi

if [ "$1" = "app-server" ]; then
  IFS= read -r ai_meter_initialize
  printf '{"id":1,"result":{"userAgent":"fake-codex"}}\n'
  IFS= read -r ai_meter_initialized
  IFS= read -r ai_meter_rate_request
  printf '{"id":2,"result":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":27,"windowDurationMins":300,"resetsAt":1900000000},"secondary":{"usedPercent":8,"windowDurationMins":10080,"resetsAt":1900600000}}}}\n'
  exit 0
fi

IFS= read -r ai_meter_command
ai_meter_command=$(printf '%s' "$ai_meter_command" | tr -d '\r\004\010')

case "$ai_meter_command" in
  fail)
    printf 'received:fail\n'
    exit 7
    ;;
  hang)
    sleep 5
    ;;
  /usage)
    printf 'Current session\n73%% used\nResets in 51 min\nAll models\n7%% used\nResets Thu 12:00 AM\n'
    ;;
  /status)
    printf '5h limit: 27%% remaining\nResets 10:30 PM\nWeekly limit: 92%% remaining\nResets Fri 1:00 AM\n'
    ;;
  *)
    printf 'unknown command\n'
    exit 2
    ;;
esac
