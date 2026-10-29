"""Hosted complete53 protocol and21 DATA GREEN; no native permission claim."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

VERSION = 1
BASE = 'bde5808e0d478053848e1d069ddf62ce9e4191b6'
REF = 'refs/heads/codex/code0-permission-initial-stop-only-green-20261030'
SOURCE = 'windows/src-tauri/src/platform/windows/code0_webview/permission_protocol.rs'
SELF = 'scripts/test-code0-permission-initial-stop-only-green.py'
WORKFLOW = '.github/workflows/code0-permission-initial-stop-only-green.yml'
VALIDATOR = '.ci/code0-permission-checker/validator.py'
SELFCHECK = '.ci/code0-permission-checker/selfcheck.py'
PATHS = {SOURCE, VALIDATOR, SELFCHECK, SELF, WORKFLOW}
SOURCE_PINS = {'windows/src-tauri/src/platform/windows/code0_webview/permission_protocol.rs': {'bytes': 118412, 'sha256': '9db326be50cd117cfcd7d3eff2f418eabc49447c3bd1ba9923af18ee9d5c6fb1'}, '.ci/code0-permission-checker/validator.py': {'bytes': 25162, 'sha256': '45aced4f4bf2ab8aaeec63517047ad51fefde6738a5503e7bb83f4667da6dec5'}, '.ci/code0-permission-checker/selfcheck.py': {'bytes': 31534, 'sha256': 'd4580e347ba799b0f2f0c0b87e23a7cb39caae10a78260f0cee682e8a93995f7'}}
BASELINE_PINS = {'windows/src-tauri/src/platform/windows/code0_webview/permission_protocol.rs': {'bytes': 82833, 'sha256': '1b4b803e28e4a4a52ff76d345f85c25fbfa68bbe1c9c31117e17735c22ba35d3'}, '.ci/code0-permission-checker/validator.py': {'bytes': 25108, 'sha256': 'c8d0ca1b75829a2747dec8e0f07f855e8d3c3cd38e59ece9a79e5658223b6ddf'}, '.ci/code0-permission-checker/selfcheck.py': {'bytes': 28179, 'sha256': 'fabdbc6509e690d8557b8fff56e0527287fa4fac3190242bc9f66ed927de63bf'}}
EXPECTED_PROTOCOL = ['native_clock_fix2_tests::unavailable_native_clock_is_negative_and_retains_last_known_semantic_time', 'native_failure_fix2_tests::adapter_failure_revokes_without_inventing_delivery_ticket_or_restarting_close', 'tests::affinity_loss_has_no_invented_ui_boundary_and_is_permanent', 'tests::commands_are_ordered_once_only_and_require_consumed_ack', 'tests::counters_saturate_with_an_explicit_overflow_bit', 'tests::coverage_requires_every_real_registration_and_zero_token_is_owned', 'tests::cutoff_crossing_capture_or_postdrop_rejects_effect_but_disposes_obligation', 'tests::effect_invocation_before_native_action_never_qualifies', 'tests::every_receiver_invalidates_after_arm_and_delivery_is_counted', 'tests::fully_qualified_historical_receipt_survives_later_cutoff_only_as_history', 'tests::genuine_denied_completion_can_qualify_while_closing_without_reauthorization', 'tests::handler_drop_without_invocation_preserves_obligation_and_no_receipt', 'tests::historical_receipt_consumption_after_original_deadline_is_not_a_new_positive', 'tests::hresult_and_closed_denied_value_are_independent_requirements', 'tests::identifiers_and_context_ids_are_bounded', 'tests::initial_orphan_stop_and_start_only_require_their_missing_facts', 'tests::initial_stop_only_green_tests::all_prefixes_and_24_selected_orders_reach_checked_effect_and_protocol_release', 'tests::initial_stop_only_green_tests::effect_needs_native_action_successful_closed_result_and_pre_cutoff_qualification', 'tests::initial_stop_only_green_tests::every_native_navigation_and_response_input_remains_mandatory', 'tests::initial_stop_only_green_tests::initial_duplicates_wrong_root_and_unsupported_events_are_sticky', 'tests::initial_stop_only_green_tests::missing_any_selected_fact_blocks_tree_source_arm_and_evaluate', 'tests::initial_stop_only_green_tests::missing_effect_never_drains_and_native_failure_never_qualifies', 'tests::initial_stop_only_green_tests::missing_retirement_blocks_until_actual_whole_pair_destroy', 'tests::initial_stop_only_green_tests::old_empty_and_observed_none_or_full_pair_classifications_stay_exact', 'tests::initial_stop_only_green_tests::original_and_close_equality_and_physical_entry_keep_their_negative_meanings', 'tests::initial_stop_only_green_tests::orphan_stop_or_frame_never_selects_zero_or_stop_only', 'tests::initial_stop_only_green_tests::real_redundant_retirement_signals_keep_tombstone_rules', 'tests::initial_stop_only_green_tests::reservation_and_postarm_lifecycle_are_once_only', 'tests::initial_stop_only_green_tests::reservation_keeps_current_identity_coverage_cutoff_and_original45', 'tests::initial_stop_only_green_tests::selected_identity_retirement_and_late_blank_rules_are_unchanged', 'tests::initial_stop_only_green_tests::tree_and_both_source_boundaries_cannot_be_substituted', 'tests::initial_stop_only_green_tests::unconsumed_or_failed_enable_ack_cannot_reserve', 'tests::initial_stop_only_red_tests::context_then_stop_with_blank_frame_reserves_navigation', 'tests::initial_stop_only_red_tests::context_then_stop_without_blank_frame_reserves_navigation', 'tests::initial_stop_only_red_tests::stop_then_context_with_blank_frame_reserves_navigation', 'tests::initial_stop_only_red_tests::stop_then_context_without_blank_frame_reserves_navigation', 'tests::main_context_frame_and_loads_join_in_every_relative_order', 'tests::native_checked_action_requires_real_order_and_all_independent_facts', 'tests::observed_blank_identity_is_not_reassigned', 'tests::origin_allowlist_and_full_document_source_have_separate_meanings', 'tests::original_deadline_equality_is_late_and_first_close_never_restarts', 'tests::original_late_error_completion_drains_within_original_first_close_budget', 'tests::physical_entry_before_qualification_defeats_new_positive_evidence', 'tests::registration_phase_has_no_bootstrap_exception', 'tests::removal_reservation_latches_cutoff_before_exposing_native_token', 'tests::selected_context_before_reservation_is_rejected', 'tests::selected_context_clear_and_wrong_unique_id_destroy_fail', 'tests::selected_unique_context_peek_is_copy_only_and_reserves_nothing', 'tests::selected_unique_context_peek_never_authorizes_after_source_time_or_cutoff_loss', 'tests::stale_released_and_close_deadline_deliveries_cannot_drain', 'tests::tombstone_destroy_is_once_only_and_never_grandfathered_after_arm', 'tests::unexpected_coverage_loss_is_sticky_and_never_repairs', 'tests::zero_blank_branch_is_sealed_and_main_load_pair_is_separate']
EXPECTED_DATA = [('PermissionDataChecks', 'test_all_four_command_rows_and_obligations_are_independent'), ('PermissionDataChecks', 'test_both_branches_and_nonconstant_legal_observations'), ('PermissionDataChecks', 'test_bounded_failure_shape_is_inspectable_but_never_accepted'), ('PermissionDataChecks', 'test_deadline_order_and_rounding_negatives'), ('PermissionDataChecks', 'test_disposal_S16_endpoints_and_run_runtime_limits'), ('PermissionDataChecks', 'test_each_coverage_row_and_mask_is_mandatory'), ('PermissionDataChecks', 'test_every_exact_once_count_zero_and_two_without_losing_pass'), ('PermissionDataChecks', 'test_every_object_keyset_duplicate_and_every_array_length'), ('PermissionDataChecks', 'test_every_required_leaf_unavailable_or_wrong_type_is_rejected'), ('PermissionDataChecks', 'test_fresh_run_exit_runtime_and_no_other_schema_fallback'), ('PermissionDataChecks', 'test_inventory_bounds_overflow_and_coupled_records'), ('PermissionDataChecks', 'test_peak_history_and_normal_cutoff_ranges'), ('PermissionDataChecks', 'test_precise_flags_allow_rounded_equalities_and_final_after45'), ('PermissionDataChecks', 'test_raw_JSON_duplicate_depth_number_and_encoding_negatives'), ('PermissionDataChecks', 'test_reader_requests_only8193_and_rejects_the_extra_byte'), ('PermissionDataChecks', 'test_real_cleanup_and_P1_shared_semantics_cannot_be_substituted'), ('PermissionDataChecks', 'test_saturated_partial_counts_preserve_truthful_lower_bounds'), ('PermissionDataChecks', 'test_stop_only_closed_branch_and_realistic_synthetic_histories'), ('PermissionDataChecks', 'test_stop_only_literal_never_substitutes_missing_navigation_target_or_effect'), ('PermissionDataChecks', 'test_stop_only_still_requires_every_existing_positive_leaf'), ('PermissionDataChecks', 'test_strict_B_requires_every_native_effect_session_and_final_receipt')]
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
    expected_changes = sorted([['A', SELF], ['A', WORKFLOW], ['M', SOURCE], ['M', VALIDATOR], ['M', SELFCHECK]])
    if head != identity['checkout'] or head != os.environ['GITHUB_SHA'] or parents != [head, BASE]:
        raise ValueError('wrong_exact_checkout_or_parent')
    if sorted(changes) != expected_changes or set(identity['payloads']) != PATHS:
        raise ValueError('wrong_five_path_delta_or_payload_set')
    for name, expected in BASELINE_PINS.items():
        baseline = subprocess.check_output(['git', 'show', BASE + ':' + name], cwd=root)
        if pin(baseline) != expected:
            raise ValueError('wrong_fixed_source_baseline')
    for name in sorted(PATHS):
        raw = (root / name).read_bytes()
        blob = subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=root)
        mode = subprocess.check_output(['git', 'ls-tree', 'HEAD', '--', name], cwd=root, text=True).split()[0]
        if raw != blob or mode != '100644' or pin(raw) != identity['payloads'][name]:
            raise ValueError('payload_changed')
        if name in SOURCE_PINS and pin(raw) != SOURCE_PINS[name]:
            raise ValueError('frozen_source_changed')
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
                                     env={**os.environ, 'RUST_BACKTRACE': '0', 'PYTHONDONTWRITEBYTECODE': '1'})
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


def exact_green(stage, stdout, stderr):
    rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', stdout, re.MULTILINE)
    names = [name for name, status in rows]
    summaries = re.findall(r'^test result: ok\. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9]+(?:\.[0-9]+)?s$', stdout, re.MULTILINE)
    if (stage['exitCode'] != 0 or not stage['childReaped'] or stderr
            or len(rows) != 53 or len(set(names)) != 53
            or sorted(names) != EXPECTED_PROTOCOL or any(status != 'ok' for _, status in rows)
            or len(re.findall(r'^test (?!result:)', stdout, re.MULTILINE)) != 53
            or len(summaries) != 1):
        raise ValueError('wrong_complete53_protocol_green')
    expected = ['running 53 tests'] + [f'test {name} ... ok' for name in names] + summaries
    if [line for line in stdout.splitlines() if line] != expected:
        raise ValueError('unexpected_complete_protocol_stdout')
    return sorted(names), []


def exact_data(stage, stdout, stderr):
    rows = re.findall(r'^([A-Za-z0-9_]+) \(__main__\.([A-Za-z0-9_]+)\.\1\) \.\.\. ok$', stderr, re.MULTILINE)
    actual = [(klass, name) for name, klass in rows]
    summaries = re.findall(r'^Ran 21 tests in [0-9]+(?:\.[0-9]+)?s$', stderr, re.MULTILINE)
    if (stage['exitCode'] != 0 or not stage['childReaped'] or stdout
            or len(actual) != 21 or len(set(actual)) != 21
            or sorted(actual) != EXPECTED_DATA or len(summaries) != 1):
        raise ValueError('wrong_complete21_data_green')
    expected = [f'{name} (__main__.{klass}.{name}) ... ok' for klass, name in actual]
    expected += ['-' * 70] + summaries + ['OK']
    if [line for line in stderr.splitlines() if line] != expected:
        raise ValueError('unexpected_complete_data_stderr')
    return [klass + '.' + name for klass, name in sorted(actual)]


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
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-permission-initial-stop-only-green-{run}-{attempt}'
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
             'passed': [], 'semanticFailures': [], 'dataPassed': [], 'childReaped': False}
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
        state['passed'], state['semanticFailures'] = exact_green(stage, stdout, stderr)
        if pin(binary.read_bytes()) != state['testBinary']:
            raise ValueError('compiled_test_binary_changed')
        check_payloads(root, identity)
        if time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('final_collector_deadline')
        stage['result'] = 'complete53_protocol_green'
        stage, stdout, stderr = run_stage(root, out, state, 'complete-data',
            [sys.executable, '-B', SELFCHECK, '-v'], 30, started, identity)
        state['dataPassed'] = exact_data(stage, stdout, stderr)
        stage['result'] = 'complete21_data_green'
        check_payloads(root, identity)
        if time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('final_data_deadline')
        state['result'] = 'complete_protocol_and_data_initial_stop_only_green'
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
    if state['result'] != 'complete_protocol_and_data_initial_stop_only_green' or not state['childReaped']:
        raise SystemExit('Exact complete53 protocol and21 DATA GREEN was not established')
    print('Observed all53 protocol and21 synthetic DATA PASS; native permission, selected-document reachability, effect and cleanup remain unestablished.')


if __name__ == '__main__':
    main()
