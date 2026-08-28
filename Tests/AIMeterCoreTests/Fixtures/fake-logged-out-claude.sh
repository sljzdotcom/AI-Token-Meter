#!/bin/sh

if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}\n'
  # Claude CLI uses a non-zero exit status to represent a logged-out account,
  # even though it still returns a valid machine-readable status document.
  exit 1
fi

printf 'interactive mode must not start while logged out\n'
exit 9
