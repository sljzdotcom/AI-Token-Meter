"""Private hosted std-only RED collector; no native permission or GREEN claim."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

VERSION = 1
BASE = '2cfbe796f60dbaef287f0a133e03bbafa7dd0dfb'
REF = 'refs/heads/codex/code0-permission-initial-stop-only-red-20261030'
SOURCE = 'windows/src-tauri/src/platform/windows/code0_webview/permission_protocol.rs'
SELF = 'scripts/test-code0-permission-initial-stop-only-red.py'
WORKFLOW = '.github/workflows/code0-permission-initial-stop-only-red.yml'
PATHS = {SOURCE, SELF, WORKFLOW}
SOURCE_PIN = {'bytes': 82833, 'sha256': '1b4b803e28e4a4a52ff76d345f85c25fbfa68bbe1c9c31117e17735c22ba35d3'}
BASELINE_PIN = {'bytes': 80095, 'sha256': 'a8fe9d8a95d0749f5485f45e61c39a544ec23ff5341e7bccdeb1e74ed7ba3ea6'}
OLD_PASSED = ['native_clock_fix2_tests::unavailable_native_clock_is_negative_and_retains_last_known_semantic_time', 'native_failure_fix2_tests::adapter_failure_revokes_without_inventing_delivery_ticket_or_restarting_close', 'tests::affinity_loss_has_no_invented_ui_boundary_and_is_permanent', 'tests::commands_are_ordered_once_only_and_require_consumed_ack', 'tests::counters_saturate_with_an_explicit_overflow_bit', 'tests::coverage_requires_every_real_registration_and_zero_token_is_owned', 'tests::cutoff_crossing_capture_or_postdrop_rejects_effect_but_disposes_obligation', 'tests::effect_invocation_before_native_action_never_qualifies', 'tests::every_receiver_invalidates_after_arm_and_delivery_is_counted', 'tests::fully_qualified_historical_receipt_survives_later_cutoff_only_as_history', 'tests::genuine_denied_completion_can_qualify_while_closing_without_reauthorization', 'tests::handler_drop_without_invocation_preserves_obligation_and_no_receipt', 'tests::historical_receipt_consumption_after_original_deadline_is_not_a_new_positive', 'tests::hresult_and_closed_denied_value_are_independent_requirements', 'tests::identifiers_and_context_ids_are_bounded', 'tests::initial_loading_half_pair_prevents_reservation_until_resolved', 'tests::main_context_frame_and_loads_join_in_every_relative_order', 'tests::native_checked_action_requires_real_order_and_all_independent_facts', 'tests::observed_blank_identity_is_not_reassigned', 'tests::origin_allowlist_and_full_document_source_have_separate_meanings', 'tests::original_deadline_equality_is_late_and_first_close_never_restarts', 'tests::original_late_error_completion_drains_within_original_first_close_budget', 'tests::physical_entry_before_qualification_defeats_new_positive_evidence', 'tests::registration_phase_has_no_bootstrap_exception', 'tests::removal_reservation_latches_cutoff_before_exposing_native_token', 'tests::selected_context_before_reservation_is_rejected', 'tests::selected_context_clear_and_wrong_unique_id_destroy_fail', 'tests::selected_unique_context_peek_is_copy_only_and_reserves_nothing', 'tests::selected_unique_context_peek_never_authorizes_after_source_time_or_cutoff_loss', 'tests::stale_released_and_close_deadline_deliveries_cannot_drain', 'tests::tombstone_destroy_is_once_only_and_never_grandfathered_after_arm', 'tests::unexpected_coverage_loss_is_sticky_and_never_repairs', 'tests::zero_blank_branch_is_sealed_and_main_load_pair_is_separate']
RED_NAMES = ['tests::initial_stop_only_red_tests::context_then_stop_without_blank_frame_reserves_navigation', 'tests::initial_stop_only_red_tests::stop_then_context_without_blank_frame_reserves_navigation', 'tests::initial_stop_only_red_tests::context_then_stop_with_blank_frame_reserves_navigation', 'tests::initial_stop_only_red_tests::stop_then_context_with_blank_frame_reserves_navigation']
ASSERTION = {'path': SOURCE, 'line': 2165, 'column': 13,
             'message': 'IB-R observed initial stop-only prefix must reserve without fabricating start',
             'left': 'Err(NotReady)', 'right': 'Ok(())'}
LOG_CAP = 1048576
TOTAL_SECONDS = 300


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
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    parents = subprocess.check_output(['git', 'rev-list', '--parents', '-n', '1', 'HEAD'], cwd=root, text=True).split()
    changes = [line.split('\t') for line in subprocess.check_output(
        ['git', 'diff-tree', '--no-commit-id', '--name-status', '-r', BASE, 'HEAD'],
        cwd=root, text=True).splitlines()]
    expected_changes = sorted([['A', SELF], ['A', WORKFLOW], ['M', SOURCE]])
    if head != identity['checkout'] or head != os.environ['GITHUB_SHA'] or parents != [head, BASE]:
        raise ValueError('wrong_exact_checkout_or_parent')
    if sorted(changes) != expected_changes or set(identity['payloads']) != PATHS:
        raise ValueError('wrong_three_path_delta_or_payload_set')
    baseline = subprocess.check_output(['git', 'show', BASE + ':' + SOURCE], cwd=root)
    if pin(baseline) != BASELINE_PIN:
        raise ValueError('wrong_fixed_protocol_baseline')
    for name in sorted(PATHS):
        raw = (root / name).read_bytes()
        blob = subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=root)
        mode = subprocess.check_output(['git', 'ls-tree', 'HEAD', '--', name], cwd=root, text=True).split()[0]
        if raw != blob or mode != '100644' or pin(raw) != identity['payloads'][name]:
            raise ValueError('payload_changed')
        if name == SOURCE and pin(raw) != SOURCE_PIN:
            raise ValueError('frozen_protocol_changed')
    if subprocess.check_output(['git', 'diff', '--name-only', 'HEAD'], cwd=root, text=True).strip():
        raise ValueError('tracked_working_input_changed')


def run_stage(root, out, state, name, argv, seconds, started, identity):
    check_payloads(root, identity)
    stage = {'name': name, 'argv': argv, 'cwd': 'checkout-root', 'exitCode': None,
             'childReaped': False, 'result': 'incomplete', 'error': 'none'}
    state['stages'].append(stage)
    paths = [out / (name + '.stdout.log'), out / (name + '.stderr.log')]
    stage_started = time.monotonic()
    try:
        with paths[0].open('xb') as stdout, paths[1].open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=root, stdin=subprocess.DEVNULL,
                                     stdout=stdout, stderr=stderr, shell=False,
                                     env={**os.environ, 'RUST_BACKTRACE': '0'})
            while child.poll() is None:
                if (time.monotonic() - stage_started >= seconds
                        or time.monotonic() - started >= TOTAL_SECONDS
                        or any(path.stat().st_size > LOG_CAP for path in paths)):
                    # No signal/retry/hash of active logs or invented reap receipt.
                    raise ValueError('active_stage_deadline_or_overflow')
                time.sleep(0.05)
            stage['exitCode'] = child.wait()
            stage['childReaped'] = True
        if (time.monotonic() - stage_started >= seconds
                or time.monotonic() - started >= TOTAL_SECONDS
                or any(path.stat().st_size > LOG_CAP for path in paths)):
            raise ValueError('completed_stage_deadline_or_overflow')
        stage['logs'] = {path.name: pin(path.read_bytes()) for path in paths}
        text = [path.read_text(encoding='utf-8') for path in paths]
        check_payloads(root, identity)
        if time.monotonic() - stage_started >= seconds or time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('post_guard_deadline')
        stage['result'] = 'returned'
        return stage, text[0], text[1]
    except (OSError, ValueError, UnicodeError) as error:
        stage['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
        raise
    finally:
        stage['elapsedSeconds'] = round(time.monotonic() - stage_started, 3)


def exact_red(stage, stdout, stderr):
    rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', stdout, re.MULTILINE)
    passed = sorted(name for name, status in rows if status == 'ok')
    failed = sorted(name for name, status in rows if status == 'FAILED')
    summaries = re.findall(r'^test result: FAILED\. 33 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9]+(?:\.[0-9]+)?s$', stdout, re.MULTILINE)
    if (stage['exitCode'] != 101 or not stage['childReaped']
            or len(rows) != 37 or len(set(name for name, _ in rows)) != 37
            or len(re.findall(r'^test (?!result:)', stdout, re.MULTILINE)) != 37
            or passed != sorted(OLD_PASSED) or failed != sorted(RED_NAMES)
            or len(re.findall(r'^running [0-9]+ tests$', stdout, re.MULTILINE)) != 1
            or len(re.findall(r'^running 37 tests$', stdout, re.MULTILINE)) != 1
            or len(re.findall(r'^test result:', stdout, re.MULTILINE)) != 1
            or len(summaries) != 1 or 'panicked at' in stdout):
        raise ValueError('wrong_complete_protocol_results')
    failure_list = 'failures:\n\nfailures:\n' + ''.join('    ' + name + '\n' for name in sorted(RED_NAMES))
    expected_lines = (['running 37 tests']
                      + [f'test {name} ... {status}' for name, status in rows]
                      + ['failures:', 'failures:']
                      + ['    ' + name for name in sorted(RED_NAMES)] + summaries)
    if stdout.count(failure_list) != 1 or [line for line in stdout.splitlines() if line] != expected_lines:
        raise ValueError('unexpected_stdout_or_failure_list')
    text = stderr.replace('\\', '/')
    headers = list(re.finditer(r"^thread '([^']+)' panicked at ([^\n]+):\n", text, re.MULTILINE))
    if (text.count('panicked at') != 4 or len(headers) != 4
            or text[:headers[0].start()].strip()):
        raise ValueError('wrong_exact_panic_header_count_or_prefix')
    note = 'note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n'
    if text.count(note) > 1:
        raise ValueError('duplicate_backtrace_note')
    concrete = ("assertion `left == right` failed: " + ASSERTION['message'] + '\n'
                + '  left: ' + ASSERTION['left'] + '\n'
                + ' right: ' + ASSERTION['right'] + '\n')
    names = set()
    for index, header in enumerate(headers):
        name, location = header.groups()
        if name in names or name not in RED_NAMES:
            raise ValueError('unexpected_or_duplicate_panic')
        names.add(name)
        suffix = f"{ASSERTION['path']}:{ASSERTION['line']}:{ASSERTION['column']}"
        if not (location == suffix or location.endswith('/' + suffix)):
            raise ValueError('wrong_real_assertion_location')
        end = headers[index + 1].start() if index + 1 < len(headers) else len(text)
        if re.fullmatch(re.escape(concrete) + '(?:' + re.escape(note) + ')?\n*', text[header.end():end]) is None:
            raise ValueError('wrong_concrete_semantic_panic')
    if names != set(RED_NAMES):
        raise ValueError('missing_panic_identity')
    return passed, failed


def main():
    if (os.name != 'posix' or os.environ.get('RUNNER_OS') != 'Linux'
            or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'
            or os.environ.get('GITHUB_EVENT_NAME') != 'push'
            or os.environ.get('GITHUB_REF') != REF):
        raise SystemExit('Requires the exact isolated hosted Ubuntu push')
    root = Path(os.environ['GITHUB_WORKSPACE'])
    if Path.cwd() != root or len(sys.argv) != 1 or Path(__file__).resolve() != (root / SELF).resolve():
        raise SystemExit('Unexpected collector workspace/path/arguments')
    run, attempt, sha = (os.environ[n] for n in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_SHA'))
    if (re.fullmatch(r'[0-9]{1,20}', run) is None or re.fullmatch(r'[0-9]{1,6}', attempt) is None
            or re.fullmatch(r'[0-9a-f]{40}', sha) is None):
        raise SystemExit('Invalid immutable run identity')
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-permission-initial-stop-only-red-{run}-{attempt}'
    raw = (out / 'identity.json').read_bytes()
    if len(raw) > 16384:
        raise SystemExit('Oversize identity')
    identity = json.loads(raw.decode('utf-8'), object_pairs_hook=unique)
    keys = {'schema', 'runId', 'attempt', 'checkout', 'parent', 'ref', 'event', 'payloads', 'runnerHost'}
    if (type(identity) is not dict or set(identity) != keys or type(identity.get('schema')) is not int
            or identity['schema'] != 1 or identity['runId'] != run or identity['attempt'] != attempt
            or identity['checkout'] != sha or identity['parent'] != BASE
            or identity['ref'] != REF or identity['event'] != 'push'
            or type(identity['payloads']) is not dict or type(identity['runnerHost']) is not dict):
        raise SystemExit('Mismatched immutable identity')
    state = {'schema': 1, 'collectorVersion': VERSION, 'runId': run, 'attempt': attempt,
             'checkout': sha, 'base': BASE, 'identity': pin(raw), 'payloads': identity['payloads'],
             'runnerHost': identity['runnerHost'], 'argv': [sys.executable, *sys.argv],
             'python': sys.version, 'result': 'incomplete', 'error': 'none', 'stages': [],
             'passed': [], 'semanticFailures': [], 'childReaped': False}
    started = time.monotonic()
    try:
        for name, argv, prefix in (
                ('rust-version', ['rustc', '+1.88.0', '-Vv'], 'rustc 1.88.0 ('),
                ('rustfmt-version', ['rustfmt', '+1.88.0', '--version'], 'rustfmt ')):
            stage, stdout, stderr = run_stage(root, out, state, name, argv, 30, started, identity)
            if (stage['exitCode'] != 0 or not stdout.startswith(prefix) or stderr
                    or len(stdout.encode('utf-8')) > 8192 or 'panicked at' in stdout):
                raise ValueError('tool_identity_failed')
            stage['result'] = 'version_verified'
        stage, stdout, stderr = run_stage(root, out, state, 'strict-format',
            ['rustfmt', '+1.88.0', '--check', '--edition', '2024', '--config', 'skip_children=true', SOURCE],
            30, started, identity)
        if stage['exitCode'] != 0 or stdout or stderr:
            raise ValueError('strict_format_did_not_pass')
        stage['result'] = 'gate_passed'
        binary = out / 'permission-initial-stop-only-tests'
        stage, stdout, stderr = run_stage(root, out, state, 'strict-compile',
            ['rustc', '+1.88.0', '--edition=2024', '--deny', 'warnings', '--test', SOURCE, '-o', str(binary)],
            90, started, identity)
        if (stage['exitCode'] != 0 or stdout or stderr or not binary.is_file()
                or binary.stat().st_size > 33554432):
            raise ValueError('strict_compile_did_not_pass')
        state['testBinary'] = pin(binary.read_bytes())
        stage['result'] = 'gate_passed'
        stage, stdout, stderr = run_stage(root, out, state, 'complete-protocol',
            [str(binary), '--nocapture', '--test-threads=1'], 30, started, identity)
        state['passed'], state['semanticFailures'] = exact_red(stage, stdout, stderr)
        if pin(binary.read_bytes()) != state['testBinary']:
            raise ValueError('compiled_test_binary_changed')
        check_payloads(root, identity)
        if time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('final_collector_deadline')
        stage['result'] = 'old33_passed_four_exact_initial_stop_only_semantic_red'
        state['result'] = 'complete_protocol_initial_stop_only_red'
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        state['childReaped'] = bool(state['stages']) and all(s['childReaped'] for s in state['stages'])
        state['elapsedSeconds'] = round(time.monotonic() - started, 3)
        encoded = json.dumps(state, indent=2) + '\n'
        if len(encoded.encode('utf-8')) > 65536:
            raise SystemExit('Oversize summary')
        with (out / 'summary.json').open('x', encoding='utf-8', newline='\n') as output:
            output.write(encoded)
    if time.monotonic() - started >= TOTAL_SECONDS:
        raise SystemExit('Late collector return is not accepted')
    print(json.dumps(state, indent=2))
    if state['result'] != 'complete_protocol_initial_stop_only_red' or not state['childReaped']:
        raise SystemExit('Exact complete protocol semantic RED was not established')
    print('Observed old33 PASS and four exact initial-stop-only semantic RED; native/selected-document/effect/GREEN remain unestablished.')


if __name__ == '__main__':
    main()
