"""Temporary hosted-CI evidence collector for a fixed behavioral RED baseline."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

PREFIX = 'platform::windows::code0_clock::tests::'
FAILURES = {
    'elapsed_milliseconds_include_exact_299_300_and_large_u64_values': ('0ns', '299s'),
    'clones_observe_the_same_live_source_without_reset_or_snapshot': ('0ns', '1.5s'),
    'manual_sync_has_one_second_remaining_at_299_and_starts_at_300':
        ('Throttled { seconds_remaining: 300 }', 'Throttled { seconds_remaining: 1 }'),
    'automatic_sync_starts_at_300_only_through_the_real_session': ('1', '2'),
    'injected_suspend_jump_coalesces_one_due_attempt_without_a_real_sleep': ('1', '2'),
}
CONTROLS = {
    'wall_jump_is_independent_and_has_native_rfc3339_values',
    'formatting_failure_yields_invalid_time_and_real_session_rejects_before_save',
}
BASELINE_SHA256 = '2224f786b606821bd2774d3cf296482dabd43334cdb5d2e8ceabde57b544536a'


def pin(path):
    raw = path.read_bytes()
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def main():
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'):
        raise SystemExit('Clock RED execution requires the isolated hosted Windows job')
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
    if source_pin != {'bytes': 12429, 'sha256': BASELINE_SHA256}:
        raise SystemExit('Clock baseline source changed')
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-clock-red-{run}-{attempt}'
    out.mkdir(exist_ok=False)
    state = {'schema': 1, 'runId': run, 'attempt': attempt, 'checkout': sha,
             'baseline': source_pin, 'result': 'incomplete', 'childReaped': False,
             'exitCode': None, 'semanticFailures': [], 'controls': [], 'error': 'none'}
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
                    # failure is not a semantic RED or a cleanup confirmation.
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
        observed_failures = [name for name, result in rows if result == 'FAILED']
        observed_controls = [name for name, result in rows if result == 'ok']
        if (state['exitCode'] != 101 or len(rows) != 7
                or sorted(observed_failures) != sorted(PREFIX + name for name in FAILURES)
                or sorted(observed_controls) != sorted(PREFIX + name for name in CONTROLS)
                or len(re.findall(r'^running 7 tests$', stdout, re.MULTILINE)) != 1
                or len(re.findall(r'^test result: FAILED\. 2 passed; 5 failed; 0 ignored; 0 measured; 190 filtered out; finished in ', stdout, re.MULTILINE)) != 1):
            raise ValueError('wrong_selected_results')
        # Bind each failure to its own test, same baseline source and the
        # concrete wrong-value assertion. Compile/setup/panic alone is not RED.
        panics = re.split(r"\n(?=thread ')", '\n' + stderr.replace('\\', '/'))
        blocks = {}
        for block in panics:
            match = re.search(r"^\n?thread '([^']+)' panicked at ([^\n]+):\n", block)
            if match:
                name, location = match.groups()
                if name in blocks:
                    raise ValueError('duplicate_test_panic')
                blocks[name] = (location, block)
        if set(blocks) != {PREFIX + name for name in FAILURES}:
            raise ValueError('wrong_panic_identities')
        for name, (left, right) in FAILURES.items():
            location, block = blocks[PREFIX + name]
            if (re.fullmatch(r'(?:.*?/)?src/platform/windows/code0_clock\.rs:[0-9]+:[0-9]+', location) is None
                    or 'assertion `left == right` failed\n' not in block
                    or re.search(r'^\s*left: ' + re.escape(left) + r'\s*$', block, re.MULTILINE) is None
                    or re.search(r'^\s*right: ' + re.escape(right) + r'\s*$', block, re.MULTILINE) is None):
                raise ValueError('wrong_semantic_assertion')
        state['semanticFailures'] = sorted(observed_failures)
        state['controls'] = sorted(observed_controls)
        state['result'] = 'expected_behavioral_red'
    except (OSError, ValueError, UnicodeError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        state['elapsedSeconds'] = round(time.monotonic() - started, 3)
        with (out / 'summary.json').open('x', encoding='utf-8', newline='\n') as output:
            json.dump(state, output, indent=2)
            output.write('\n')
    print(json.dumps(state, indent=2))
    if state['result'] != 'expected_behavioral_red':
        raise SystemExit('Clock baseline did not establish the five expected behavioral failures')
    print('Clock baseline RED observed; production clock and native suspend remain unverified.')


if __name__ == '__main__':
    main()
