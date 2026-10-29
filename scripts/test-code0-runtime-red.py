"""Temporary hosted Windows collector: strict build gates, then three exact runtime REDs."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

COLLECTOR_VERSION = 1
BASE = 'f53e3da12a1aaebe5e49b2cb1f03109d10a9e28f'
REF = 'refs/heads/codex/code0-runtime-red-format-20261029'
PREFIX = 'platform::windows::code0_runtime::tests::'
SOURCE = 'windows/src-tauri/src/platform/windows/code0_runtime.rs'
MOD = 'windows/src-tauri/src/platform/windows/mod.rs'
SELF = 'scripts/test-code0-runtime-red.py'
WORKFLOW = '.github/workflows/code0-runtime-red.yml'
PATHS = {SOURCE, MOD, SELF, WORKFLOW}
SOURCE_PINS = {
    SOURCE: {'bytes': 30963, 'sha256': 'dce9d22b50f3f9272681e1d31db376f94ab85d3ff9a9f685068158780cd76fd5'},
    MOD: {'bytes': 696, 'sha256': 'f1b5ca5f97a68e4a7d1df1087d64a66c37fb91275b5e08b312ad8fd99eb9e5bf'},
}
FAILURES = {
    'settled_worker_action_replies_on_the_ui_owner': {
        'line': 631, 'column': 9, 'kind': 'equality',
        'message': 'settled worker action must reply through the real dispatcher',
        'left': 'None', 'right': 'Some(Ok(Action(Started)))',
    },
    'settled_read_replies_without_starting_native_work': {
        'line': 647, 'column': 9, 'kind': 'message',
        'message': 'settled read must reach real dispatcher',
    },
    'window_event_and_poll_callbacks_reach_the_real_session': {
        'line': 676, 'column': 9, 'kind': 'equality',
        'message': 'forwarded native callbacks must reach the real Session',
        'left': 'String("syncing")', 'right': '"paused"',
    },
}
CONTROLS = {
    'cold_install_passive_read_and_production_action_are_inert',
    'failed_create_keeps_its_inline_released_reservation',
    'inline_cleanup_is_reserved_before_the_backend_call',
    'reservation_failure_prevents_create_and_cleanup_calls',
    'stale_binding_cannot_mark_or_forward_to_a_replacement',
    'nested_schedule_and_window_turn_are_deferred_without_reborrowing',
    'scheduling_failure_returns_unavailable_without_forged_release',
    'committed_null_revocation_is_emitted_before_the_action_reply',
}
LOG_CAP = 4194304
TOTAL_SECONDS = 1500


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate_identity_key')
        result[key] = value
    return result


def check_payloads(root, identity):
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if actual != identity['checkout'] or actual != os.environ['GITHUB_SHA']:
        raise ValueError('checkout_changed')
    parents = subprocess.check_output(['git', 'rev-list', '--parents', '-n', '1', 'HEAD'], cwd=root, text=True).split()
    if parents != [actual, BASE]:
        raise ValueError('wrong_exact_parent')
    expected_changes = sorted([['A', SOURCE], ['M', MOD], ['A', SELF], ['A', WORKFLOW]])
    changes = [line.split('\t') for line in subprocess.check_output(
        ['git', 'diff-tree', '--no-commit-id', '--name-status', '-r', BASE, 'HEAD'], cwd=root, text=True).splitlines()]
    if sorted(changes) != expected_changes:
        raise ValueError('wrong_four_path_delta')
    if type(identity.get('payloads')) is not dict or set(identity['payloads']) != PATHS:
        raise ValueError('wrong_identity_payload_set')
    for name in sorted(PATHS):
        raw = (root / name).read_bytes()
        blob = subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=root)
        if raw != blob or pin(raw) != identity['payloads'][name]:
            raise ValueError('payload_changed')
        if name in SOURCE_PINS and pin(raw) != SOURCE_PINS[name]:
            raise ValueError('frozen_runtime_changed')
    protected = ['windows/src-tauri/src', 'windows/src-tauri/tests',
                 'windows/src-tauri/build.rs', 'windows/src-tauri/Cargo.toml',
                 'windows/src-tauri/Cargo.lock']
    dirty = subprocess.check_output(['git', 'diff', '--name-only', 'HEAD', '--', *protected], cwd=root, text=True)
    if dirty.strip():
        raise ValueError('protected_compile_input_changed')


def run_stage(root, out, state, name, argv, seconds, overall_started):
    stage = {'name': name, 'argv': argv, 'cwd': 'windows/src-tauri',
             'exitCode': None, 'childReaped': False, 'result': 'incomplete', 'error': 'none'}
    state['stages'].append(stage)
    stdout_path, stderr_path = out / f'{name}.stdout.log', out / f'{name}.stderr.log'
    started = time.monotonic()
    try:
        with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=root / 'windows/src-tauri',
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                     shell=False, env={**os.environ, 'CARGO_TERM_COLOR': 'never',
                                                       'RUST_BACKTRACE': '0'})
            while child.poll() is None:
                if (time.monotonic() - started >= seconds
                        or time.monotonic() - overall_started >= TOTAL_SECONDS
                        or stdout_path.stat().st_size > LOG_CAP
                        or stderr_path.stat().st_size > LOG_CAP):
                    # Never signal, retry, hash an active log, or claim cleanup.
                    raise ValueError('active_stage_deadline_or_overflow')
                time.sleep(0.05)
            stage['exitCode'] = child.wait()
            stage['childReaped'] = True
        if (time.monotonic() - started >= seconds
                or time.monotonic() - overall_started >= TOTAL_SECONDS
                or stdout_path.stat().st_size > LOG_CAP
                or stderr_path.stat().st_size > LOG_CAP):
            raise ValueError('completed_stage_deadline_or_overflow')
        stage['logs'] = {path.name: pin(path.read_bytes()) for path in (stdout_path, stderr_path)}
        stdout = stdout_path.read_text(encoding='utf-8')
        stderr = stderr_path.read_text(encoding='utf-8')
        stage['result'] = 'returned'
        return stage, stdout, stderr
    except (OSError, ValueError, UnicodeError) as error:
        stage['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
        raise
    finally:
        stage['elapsedSeconds'] = round(time.monotonic() - started, 3)


def exact_behavior(stage, stdout, stderr):
    rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', stdout, re.MULTILINE)
    failed = sorted(name for name, result in rows if result == 'FAILED')
    controls = sorted(name for name, result in rows if result == 'ok')
    if (stage['exitCode'] != 101 or not stage['childReaped'] or len(rows) != 11
            or failed != sorted(PREFIX + name for name in FAILURES)
            or controls != sorted(PREFIX + name for name in CONTROLS)
            or len(re.findall(r'^running 11 tests$', stdout, re.MULTILINE)) != 1
            or len(re.findall(r'^test result: FAILED\. 8 passed; 3 failed; 0 ignored; 0 measured; 190 filtered out; finished in ', stdout, re.MULTILINE)) != 1):
        raise ValueError('wrong_selected_results')
    normalized = stderr.replace('\\', '/')
    if len(re.findall(r'^thread .+ panicked at ', normalized, re.MULTILINE)) != 3:
        raise ValueError('wrong_panic_count')
    blocks = {}
    for block in re.split(r"\n(?=thread ')", '\n' + normalized):
        match = re.search(r"^\n?thread '([^']+)' panicked at ([^\n]+):\n", block)
        if match:
            name, location = match.groups()
            if name in blocks:
                raise ValueError('duplicate_test_panic')
            blocks[name] = (location, block)
    if set(blocks) != {PREFIX + name for name in FAILURES}:
        raise ValueError('wrong_panic_identities')
    for name, expected in FAILURES.items():
        location, block = blocks[PREFIX + name]
        suffix = f"src/platform/windows/code0_runtime.rs:{expected['line']}:{expected['column']}"
        if not (location == suffix or location.endswith('/' + suffix)):
            raise ValueError('wrong_assertion_location')
        message = re.escape(expected['message'])
        if expected['kind'] == 'message':
            # assert!(condition, "message") prints the custom message itself.
            good = re.search(r'^' + message + r'$', block, re.MULTILINE) is not None
        else:
            good = (re.search(r'^assertion `left == right` failed: ' + message + r'$', block, re.MULTILINE) is not None
                    and re.search(r'^\s*left: ' + re.escape(expected['left']) + r'$', block, re.MULTILINE) is not None
                    and re.search(r'^\s*right: ' + re.escape(expected['right']) + r'$', block, re.MULTILINE) is not None)
        if not good:
            raise ValueError('wrong_concrete_assertion')
    return failed, controls


def main():
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'
            or os.environ.get('GITHUB_REF') != REF
            or os.environ.get('GITHUB_EVENT_NAME') != 'push'):
        raise SystemExit('Runtime RED requires the exact isolated hosted Windows push')
    root = Path(os.environ['GITHUB_WORKSPACE'])
    if Path.cwd() != root:
        raise SystemExit('Unexpected workspace')
    if len(sys.argv) != 1 or Path(__file__).resolve() != (root / SELF).resolve():
        raise SystemExit('Unexpected collector path or arguments')
    run, attempt, sha = (os.environ[name] for name in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_SHA'))
    if (re.fullmatch(r'[0-9]{1,20}', run) is None
            or re.fullmatch(r'[0-9]{1,6}', attempt) is None
            or re.fullmatch(r'[0-9a-f]{40}', sha) is None):
        raise SystemExit('Invalid CI identity')
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-runtime-red-{run}-{attempt}'
    identity_path = out / 'identity.json'
    identity_raw = identity_path.read_bytes()
    if len(identity_raw) > 16384:
        raise SystemExit('Oversize preflight identity')
    identity = json.loads(identity_raw.decode('utf-8'), object_pairs_hook=unique)
    if (type(identity) is not dict or type(identity.get('schema')) is not int
            or identity.get('schema') != 1 or type(identity.get('runnerHost')) is not dict
            or type(identity.get('payloads')) is not dict
            or identity.get('runId') != run or identity.get('attempt') != attempt
            or identity.get('checkout') != sha or identity.get('parent') != BASE
            or identity.get('ref') != REF or identity.get('event') != 'push'):
        raise SystemExit('Mismatched immutable preflight identity')
    state = {'schema': 1, 'collectorVersion': COLLECTOR_VERSION,
             'runId': run, 'attempt': attempt, 'checkout': sha, 'base': BASE,
             'identity': pin(identity_raw), 'payloads': identity['payloads'],
             'collectorArgv': [sys.executable, *sys.argv],
             'python': sys.version, 'runnerHost': identity['runnerHost'],
             'result': 'incomplete', 'childReaped': False, 'stages': [],
             'semanticFailures': [], 'controls': [], 'toolVersions': {}, 'error': 'none'}
    started = time.monotonic()
    try:
        check_payloads(root, identity)
        for name, argv, prefix in (
                ('rust-version', ['rustc', '-Vv'], 'rustc 1.88.0 ('),
                ('cargo-version', ['cargo', '-V'], 'cargo 1.88.0 ('),
                ('clippy-version', ['cargo', 'clippy', '-V'], 'clippy 0.1.88 (')):
            stage, stdout, stderr = run_stage(root, out, state, name, argv, 30, started)
            if (stage['exitCode'] != 0 or not stdout.startswith(prefix)
                    or len(stdout.encode('utf-8')) > 8192 or len(stderr.encode('utf-8')) > 8192
                    or 'panicked at' in stderr):
                raise ValueError('tool_version_gate_failed')
            state['toolVersions'][name] = stdout.strip()
            stage['result'] = 'version_verified'
        stage, _, _ = run_stage(root, out, state, 'strict-clippy',
            ['cargo', 'clippy', '--locked', '--all-targets', '--features', 'code0-native-fixture', '--', '-D', 'warnings'], 600, started)
        if stage['exitCode'] != 0:
            raise ValueError('strict_clippy_did_not_pass')
        stage['result'] = 'gate_passed'
        check_payloads(root, identity)
        stage, _, _ = run_stage(root, out, state, 'compile-no-run',
            ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture', '--lib', '--no-run'], 600, started)
        if stage['exitCode'] != 0:
            raise ValueError('compile_no_run_did_not_pass')
        stage['result'] = 'gate_passed'
        check_payloads(root, identity)
        stage, stdout, stderr = run_stage(root, out, state, 'behavior',
            ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture', '--lib', PREFIX,
             '--', '--nocapture', '--test-threads=1'], 300, started)
        failed, controls = exact_behavior(stage, stdout, stderr)
        check_payloads(root, identity)
        if time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('final_collector_deadline')
        stage['result'] = 'three_exact_assertion_failures'
        state['semanticFailures'], state['controls'] = failed, controls
        state['result'] = 'expected_behavioral_red'
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        state['childReaped'] = bool(state['stages']) and all(stage['childReaped'] for stage in state['stages'])
        state['elapsedSeconds'] = round(time.monotonic() - started, 3)
        encoded = json.dumps(state, indent=2) + '\n'
        if len(encoded.encode('utf-8')) > 65536:
            raise SystemExit('Oversize runtime summary is not RED')
        with (out / 'summary.json').open('x', encoding='utf-8', newline='\n') as output:
            output.write(encoded)
    # A late summary write may leave an earlier snapshot, but cannot exit success.
    if time.monotonic() - started >= TOTAL_SECONDS:
        raise SystemExit('Late collector return is not expected RED')
    print(json.dumps(state, indent=2))
    if state['result'] != 'expected_behavioral_red':
        raise SystemExit('Runtime baseline did not establish the three exact behavioral failures')
    print('Runtime RED observed: eight controls and three exact assertion failures; no production/native acceptance.')


if __name__ == '__main__':
    main()
