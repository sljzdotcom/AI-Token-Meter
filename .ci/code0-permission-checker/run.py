"""Isolated DATA selfcheck only; never a native permission witness."""
from pathlib import Path
import hashlib,json,os,re,subprocess,sys,tempfile

EXPECTED = ['test_all_four_command_rows_and_obligations_are_independent', 'test_both_branches_and_nonconstant_legal_observations', 'test_bounded_failure_shape_is_inspectable_but_never_accepted', 'test_deadline_order_and_rounding_negatives', 'test_disposal_S16_endpoints_and_run_runtime_limits', 'test_each_coverage_row_and_mask_is_mandatory', 'test_every_exact_once_count_zero_and_two_without_losing_pass', 'test_every_object_keyset_duplicate_and_every_array_length', 'test_every_required_leaf_unavailable_or_wrong_type_is_rejected', 'test_fresh_run_exit_runtime_and_no_other_schema_fallback', 'test_inventory_bounds_overflow_and_coupled_records', 'test_peak_history_and_normal_cutoff_ranges', 'test_precise_flags_allow_rounded_equalities_and_final_after45', 'test_raw_JSON_duplicate_depth_number_and_encoding_negatives', 'test_reader_requests_only8193_and_rejects_the_extra_byte', 'test_real_cleanup_and_P1_shared_semantics_cannot_be_substituted', 'test_saturated_partial_counts_preserve_truthful_lower_bounds', 'test_strict_B_requires_every_native_effect_session_and_final_receipt']
if (os.environ.get('GITHUB_ACTIONS') != 'true'
        or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'
        or sys.platform != 'linux'):
    raise SystemExit('This collector is for the isolated hosted DATA job only')
root = Path(__file__).resolve().parent
out = Path(tempfile.mkdtemp(prefix='code0-permission-checker-', dir=os.environ['RUNNER_TEMP']))
actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
if actual != os.environ['GITHUB_SHA']:
    raise SystemExit('Source checkout mismatch')
result = subprocess.run([sys.executable, '-B', str(root / 'selfcheck.py'), '-v'],
                        cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                        check=False)
# These are hashes of completed, reaped-process streams. A job timeout with no
# return is failure/incomplete; no retained partial stream is called complete.
logs = {}
for label, raw in [('stdout', result.stdout), ('stderr', result.stderr)]:
    (out / (label + '.log')).write_bytes(raw)
    logs[label] = {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
text = result.stderr.decode('utf-8', errors='strict')
matched = re.findall(r'^(test_[A-Za-z0-9_]+) \(__main__\.PermissionDataChecks(?:\.test_[A-Za-z0-9_]+)?\) \.\.\. ok$', text, re.M)
ran = re.findall(r'^Ran ([0-9]+) tests? in [0-9.]+s$', text, re.M)
accepted = (result.returncode == 0 and result.stdout == b''
            and sorted(matched) == EXPECTED and len(matched) == len(set(matched))
            and ran == [str(len(EXPECTED))]
            and re.search(r'^OK$', text, re.M) is not None
            and not re.search(r'^(FAILED|FAIL:|ERROR:|OK \(skipped)', text, re.M)
            and all(v['bytes'] <= 4 * 1024 * 1024 for v in logs.values()))
record = {'schema': 1, 'scope': 'schema14 synthetic DATA checker only',
          'head': actual, 'run': os.environ['GITHUB_RUN_ID'],
          'attempt': os.environ['GITHUB_RUN_ATTEMPT'], 'python': sys.version,
          'exitCode': result.returncode, 'reaped': True,
          'expected': EXPECTED, 'observed': matched, 'logs': logs,
          'accepted': bool(accepted), 'nativePermissionAccepted': False}
(out / 'summary.json').write_text(json.dumps(record, indent=2) + '\n', encoding='utf-8')
print(json.dumps(record, sort_keys=True))
print(text)
raise SystemExit(0 if accepted else 1)
