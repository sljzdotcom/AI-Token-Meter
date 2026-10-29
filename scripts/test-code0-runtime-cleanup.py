"""Temporary hosted Windows collector: complete 217-test library with runtime and cleanup GREEN."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

COLLECTOR_VERSION = 2
BASE = 'f91e0b29e0e58231d707079a81c7cd53c7ae81c1'
REF = 'refs/heads/codex/code0-runtime-cleanup-20261029'
PREFIX = 'platform::windows::code0_web_session::cleanup_deadline_red_tests::'
GREEN_PREFIX = 'platform::windows::code0_web_session::cleanup_deadline_green_tests::'
SOURCE = 'windows/src-tauri/src/platform/windows/code0_runtime.rs'
SESSION = 'windows/src-tauri/src/platform/windows/code0_web_session.rs'
RUNTIME_PREFIX = 'platform::windows::code0_runtime::tests::'
RUNTIME_TESTS = ['cold_install_passive_read_and_production_action_are_inert', 'committed_null_revocation_is_emitted_before_the_action_reply', 'failed_create_keeps_its_inline_released_reservation', 'inline_cleanup_is_reserved_before_the_backend_call', 'nested_schedule_and_window_turn_are_deferred_without_reborrowing', 'reservation_failure_prevents_create_and_cleanup_calls', 'scheduling_failure_returns_unavailable_without_forged_release', 'settled_read_replies_without_starting_native_work', 'settled_worker_action_replies_on_the_ui_owner', 'stale_binding_cannot_mark_or_forward_to_a_replacement', 'window_event_and_poll_callbacks_reach_the_real_session']
MOD = 'windows/src-tauri/src/platform/windows/mod.rs'
SELF = 'scripts/test-code0-runtime-cleanup.py'
WORKFLOW = '.github/workflows/code0-runtime-cleanup.yml'
PATHS = {SOURCE, SESSION, MOD, SELF, WORKFLOW}
SOURCE_PINS = {'windows/src-tauri/src/platform/windows/code0_runtime.rs': {'bytes': 30994, 'sha256': '5ec1d0401aa17eeb6fef5d5459988fa94d77aa2e31de8c2b39291ce66eb36ec3'}, 'windows/src-tauri/src/platform/windows/code0_web_session.rs': {'bytes': 67713, 'sha256': '6594382b83171125d9f075bb961e8f4f8991f0db2a23744710fc8d1fd04801f2'}, 'windows/src-tauri/src/platform/windows/mod.rs': {'bytes': 696, 'sha256': 'f1b5ca5f97a68e4a7d1df1087d64a66c37fb91275b5e08b312ad8fd99eb9e5bf'}}
CLEANUP_ORIGINAL = ['ownerless_disconnect_deadline_starts_before_lease_side_effect', 'old_owner_released_near_end_does_not_reset_disconnect_deadline', 'completion_available_before_end_but_consumed_at_end_is_not_success', 'matching_cleanup_consumed_one_millisecond_before_end_succeeds', 'wrong_cleanup_identities_do_not_consume_the_matching_result', 'wrong_cleanup_identities_cannot_extend_the_original_deadline', 'stop_allows_admitted_cleanup_before_end_without_reopening_actions', 'stop_does_not_disable_the_admitted_cleanup_deadline']
CLEANUP_ADDITIONAL = ['earlier_old_close_bound_survives_disconnect_and_cleanup', 'expiry_then_late_matching_success_never_heals_or_releases', 'backwards_cleanup_clock_cannot_accept_matching_success', 'overflowing_admission_fails_before_lease_cache_or_native_work', 'cold_reads_are_inert_and_driver_observes_the_same_bound_budget', 'clear_start_error_keeps_failed_identity_without_late_healing', 'lease_return_at_end_cannot_start_a_new_cleanup_budget', 'dropped_session_revokes_reader_without_synthesizing_native_release']
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
    expected_changes = sorted([['M', SESSION], ['M', SELF], ['M', WORKFLOW]])
    changes = [line.split('\t') for line in subprocess.check_output(
        ['git', 'diff-tree', '--no-commit-id', '--name-status', '-r', BASE, 'HEAD'], cwd=root, text=True).splitlines()]
    if sorted(changes) != expected_changes:
        raise ValueError('wrong_three_path_delta')
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


def exact_library_green(stage, stdout, stderr):
    rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', stdout, re.MULTILINE)
    passed = [name for name, result in rows if result == 'ok']
    expected_groups = {
        RUNTIME_PREFIX: RUNTIME_TESTS,
        PREFIX: CLEANUP_ORIGINAL,
        GREEN_PREFIX: CLEANUP_ADDITIONAL,
    }
    if (stage['exitCode'] != 0 or not stage['childReaped'] or len(rows) != 217
            or len(passed) != 217 or len(set(passed)) != 217
            or any(result != 'ok' for _, result in rows)
            or len(re.findall(r'^running 217 tests$', stdout, re.MULTILINE)) != 1
            or len(re.findall(r'^test result: ok\. 217 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in ', stdout, re.MULTILINE)) != 1
            or 'panicked at' in stdout or 'panicked at' in stderr):
        raise ValueError('complete_library_did_not_pass')
    groups = {}
    for prefix, names in expected_groups.items():
        selected = sorted(name for name in passed if name.startswith(prefix))
        if selected != sorted(prefix + name for name in names):
            raise ValueError('wrong_exact_runtime_or_cleanup_cohort')
        groups[prefix] = selected
    return sorted(passed), groups


def main():
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'
            or os.environ.get('GITHUB_REF') != REF
            or os.environ.get('GITHUB_EVENT_NAME') != 'push'):
        raise SystemExit('Runtime/cleanup validation requires the exact isolated hosted Windows push')
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
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-runtime-cleanup-green-{run}-{attempt}'
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
             'libraryPassed': [], 'cohorts': {}, 'toolVersions': {}, 'error': 'none'}
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
        stage, stdout, stderr = run_stage(root, out, state, 'complete-library',
            ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture', '--lib',
             '--', '--nocapture', '--test-threads=1'], 300, started)
        state['libraryPassed'], state['cohorts'] = exact_library_green(stage, stdout, stderr)
        check_payloads(root, identity)
        if time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('final_collector_deadline')
        stage['result'] = 'all_217_library_tests_passed'
        state['result'] = 'complete_library_green'
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        state['childReaped'] = bool(state['stages']) and all(stage['childReaped'] for stage in state['stages'])
        state['elapsedSeconds'] = round(time.monotonic() - started, 3)
        encoded = json.dumps(state, indent=2) + '\n'
        if len(encoded.encode('utf-8')) > 65536:
            raise SystemExit('Oversize runtime/cleanup summary')
        with (out / 'summary.json').open('x', encoding='utf-8', newline='\n') as output:
            output.write(encoded)
    # A late summary write may leave an earlier snapshot, but cannot exit success.
    if time.monotonic() - started >= TOTAL_SECONDS:
        raise SystemExit('Late collector return is not accepted')
    print(json.dumps(state, indent=2))
    if state['result'] != 'complete_library_green':
        raise SystemExit('Runtime/cleanup candidate did not establish all 217 library passes')
    print('All 217 library tests passed, including original runtime11 and original/additional cleanup8+8; no production/native acceptance.')


if __name__ == '__main__':
    main()
