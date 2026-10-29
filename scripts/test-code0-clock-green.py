"""Temporary hosted-CI evidence collector for a fixed clock GREEN candidate."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

PREFIX = 'platform::windows::code0_clock::'
REQUIRED = ['tests::elapsed_milliseconds_include_exact_299_300_and_large_u64_values', 'tests::clones_observe_the_same_live_source_without_reset_or_snapshot', 'tests::manual_sync_has_one_second_remaining_at_299_and_starts_at_300', 'tests::automatic_sync_starts_at_300_only_through_the_real_session', 'tests::injected_suspend_jump_coalesces_one_due_attempt_without_a_real_sleep', 'tests::wall_jump_is_independent_and_has_native_rfc3339_values', 'tests::formatting_failure_yields_invalid_time_and_real_session_rejects_before_save', 'native_binding_tests::native_factory_reads_boot_ticks_and_independent_utc']
SOURCE_SHA256 = 'a1a4760d4573c6f268a84ce3ab18fcd9ca0d80a78fab5df8d75fad46ee5ec84a'


def pin(path):
    raw = path.read_bytes()
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def main():
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'):
        raise SystemExit('Clock GREEN execution requires the isolated hosted Windows job')
    root = Path(os.environ['GITHUB_WORKSPACE'])
    if Path.cwd() != root:
        raise SystemExit('Unexpected evidence workspace')
    run, attempt, sha = (os.environ[name] for name in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_SHA'))
    if (not re.fullmatch(r'[0-9]{1,20}', run)
            or not re.fullmatch(r'[0-9]{1,6}', attempt)
            or not re.fullmatch(r'[0-9a-f]{40}', sha)):
        raise SystemExit('Invalid CI identity')
    source = root / 'windows/src-tauri/src/platform/windows/code0_clock.rs'
    source_pin = pin(source)
    if source_pin != {'bytes': 13231, 'sha256': SOURCE_SHA256}:
        raise SystemExit('Clock candidate source changed')
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-clock-green-{run}-{attempt}'
    out.mkdir(exist_ok=False)
    state = {'schema': 1, 'runId': run, 'attempt': attempt, 'checkout': sha,
             'source': source_pin, 'result': 'incomplete', 'childReaped': False,
             'exitCode': None, 'passedTests': [], 'error': 'none'}
    argv = ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture',
            '--lib', PREFIX, '--', '--nocapture', '--test-threads=1']
    state['argv'] = argv
    stdout_path, stderr_path = out / 'stdout.log', out / 'stderr.log'
    started = time.monotonic()
    try:
        with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=root / 'windows/src-tauri',
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                     shell=False, env={**os.environ, 'CARGO_TERM_COLOR': 'never',
                                                       'RUST_BACKTRACE': '0'})
            while child.poll() is None:
                if (time.monotonic() - started >= 300
                        or stdout_path.stat().st_size > 4194304
                        or stderr_path.stat().st_size > 4194304):
                    # No signal/retry and no active-log hash. The hosted job's
                    # failure is not a GREEN result or a cleanup confirmation.
                    raise ValueError('active_process_deadline_or_overflow')
                time.sleep(0.05)
            state['exitCode'] = child.wait()
            state['childReaped'] = True
        if (time.monotonic() - started >= 300
                or stdout_path.stat().st_size > 4194304
                or stderr_path.stat().st_size > 4194304):
            raise ValueError('completed_process_deadline_or_overflow')
        state['logs'] = {p.name: pin(p) for p in (stdout_path, stderr_path)}
        stdout = stdout_path.read_text(encoding='utf-8')
        stderr = stderr_path.read_text(encoding='utf-8')
        rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', stdout, re.MULTILINE)
        expected = sorted(PREFIX + name for name in REQUIRED)
        if (state['exitCode'] != 0 or len(rows) != 8
                or sorted(name for name, result in rows if result == 'ok') != expected
                or any(result != 'ok' for _, result in rows)
                or len(re.findall(r'^running 8 tests$', stdout, re.MULTILINE)) != 1
                or len(re.findall(r'^test result: ok\. 8 passed; 0 failed; 0 ignored; 0 measured; 190 filtered out; finished in ', stdout, re.MULTILINE)) != 1
                or 'panicked at' in stderr):
            raise ValueError('wrong_selected_results')
        state['passedTests'] = expected
        state['result'] = 'clock_green'
    except (OSError, ValueError, UnicodeError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        state['elapsedSeconds'] = round(time.monotonic() - started, 3)
        with (out / 'summary.json').open('x', encoding='utf-8', newline='\n') as output:
            json.dump(state, output, indent=2)
            output.write('\n')
    print(json.dumps(state, indent=2))
    if state['result'] != 'clock_green':
        raise SystemExit('Clock candidate did not pass all eight exact selected tests')
    print('Seven unchanged clock tests and native binding passed; production composition and physical suspend remain unverified.')


if __name__ == '__main__':
    main()
