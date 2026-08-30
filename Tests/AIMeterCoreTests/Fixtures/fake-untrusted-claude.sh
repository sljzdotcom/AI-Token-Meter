#!/bin/sh

# Simulates Claude's first-run workspace trust screen.

if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"loggedIn":true,"authMethod":"oauth","apiProvider":"firstParty"}\n'
  exit 0
fi

printf 'Permission Required: Accessing workspace:\n'
printf '%s\n' "$PWD"
sleep 5
