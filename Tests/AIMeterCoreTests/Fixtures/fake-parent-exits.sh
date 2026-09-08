#!/bin/sh

trace_path="${1:-}"
mark() {
    if [ -n "$trace_path" ]; then
        printf '%s|%s\n' "$1" "$2" >> "$trace_path" 2>/dev/null || true
    fi
}

mark shell_started "$$"
printf 'parent-exited\n'
mark parent_output_flushed "$$"

# Keep the PTY slave open after this parent exits. Ignoring HUP prevents the
# noninteractive shell from ending the synthetic descendant with its parent.
(
    trap '' HUP
    sleep 6
) &
child_pid=$!
mark child_spawned "$child_pid"
mark parent_exit_requested "$$"
exit 0
