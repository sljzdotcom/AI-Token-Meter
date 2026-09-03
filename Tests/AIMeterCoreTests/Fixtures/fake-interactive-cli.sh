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
  case "$ai_meter_rate_request" in
    *rateLimits*)
      printf '{"id":2,"result":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":27,"windowDurationMins":300,"resetsAt":1900000000},"secondary":{"usedPercent":8,"windowDurationMins":10080,"resetsAt":1900600000}}}}\n'
      ;;
    *account*read*)
      case "${AI_METER_TEST_ACCOUNT_KIND:-chatgpt}" in
        api-key)
          printf '{"id":2,"result":{"account":{"type":"apiKey"},"requiresOpenaiAuth":false}}\n'
          ;;
        signed-out)
          printf '{"id":2,"result":{"account":null,"requiresOpenaiAuth":true}}\n'
          ;;
        invalid)
          printf '{"id":2,"result":{"unexpected":true}}\n'
          ;;
        *)
          printf '{"id":2,"result":{"account":{"type":"chatgpt","email":"codex@example.com","planType":"pro"},"requiresOpenaiAuth":true}}\n'
          ;;
      esac
      ;;
    *) exit 3 ;;
  esac
  exit 0
fi

IFS= read -r ai_meter_command

case "$ai_meter_command" in
  fail)
    printf 'received:fail\n'
    exit 7
    ;;
  hang)
    sleep 5
    ;;
  identity)
    printf 'user:%s\n' "${USER:-missing}"
    ;;
  pwd)
    printf 'working-directory:%s\n' "$PWD"
    ;;
  trust)
    printf 'Permission Required: Accessing workspace\n'
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
