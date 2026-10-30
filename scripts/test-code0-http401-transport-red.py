"""Private hosted full-library HTTP transport RED collector; never imports product code."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

BASE = '32b611cd56b1d04942a13848adbbeb382dcddc4a'
BASE_TREE = '8349cac515c985cce0740d09aaf271183116eafa'
REF = 'refs/heads/codex/code0-http401-transport-red-20261030'
SELF = 'scripts/test-code0-http401-transport-red.py'
WORKFLOW = '.github/workflows/code0-http401-transport-red.yml'
DATA = '.ci/code0-http401-transport-red/schema-data.json'
DATA_PIN = {'bytes': 189431, 'sha256': 'd75787f3dee3024166190218797f150e31d1d38cfc4ac9b905517ea8b3fa8d3a'}
CASE = 'idle-first-request-second'
VARIANT = 'legacyOneAccept'
LOG_CAP = 4194304
TOTAL_SECONDS = 1500
STAGE_LIMITS = [30, 30, 30, 600, 600, 300]
NOTE = 'note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n'


def need(value, reason):
    if not value:
        raise ValueError(reason)


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def unique(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, 'duplicate_json_key')
        result[key] = value
    return result


def bad_number(_):
    raise ValueError('non_integer_json_number')


def decode(raw, limit):
    need(0 < len(raw) <= limit and raw.isascii() and not raw.startswith(b'\xef\xbb\xbf'),
         'json_encoding_or_cap')
    return json.loads(raw.decode('ascii'), object_pairs_hook=unique,
                      parse_float=bad_number, parse_constant=bad_number)


def compact(value):
    return json.dumps(value, ensure_ascii=True, separators=(',', ':'), allow_nan=False).encode('ascii')


def emit(out, name, value, limit):
    raw = compact(value)
    need(len(raw) <= limit, 'record_encoding_cap')
    with (out / name).open('xb') as target:
        target.write(raw)
    return raw


def typed(value, spec, definitions):
    if '$ref' in spec:
        typed(value, definitions[spec['$ref']], definitions)
        return
    kind = spec['type']
    if kind == 'closed-object':
        need(type(value) is dict and set(value) == set(spec['fields']), 'closed_object_keys')
        for key, child in spec['fields'].items():
            typed(value[key], child, definitions)
    elif kind == 'enum':
        need(any(type(value) is type(item) and value == item for item in spec['values']), 'enum_value')
    elif kind == 'integer':
        need(type(value) is int and spec['min'] <= value <= spec['max'], 'integer_range')
    elif kind == 'boolean':
        need(type(value) is bool, 'boolean_type')
    elif kind == 'nullable':
        if value is not None:
            typed(value, spec['value'], definitions)
    elif kind == 'ASCII':
        patterns = {'lowercase-hex': '[0-9a-f]*', 'A-Za-z0-9_-': '[A-Za-z0-9_-]*',
                    'A-Za-z0-9_./-': '[A-Za-z0-9_./-]*', '0-9.': '[0-9.]*'}
        need(type(value) is str and spec['minLength'] <= len(value) <= spec['maxLength']
             and value.isascii() and re.fullmatch(patterns[spec['alphabet']], value) is not None,
             'ascii_string')
    elif kind == 'fixed-array':
        need(type(value) is list and len(value) == len(spec['items']), 'fixed_array')
        for item, child in zip(value, spec['items']):
            typed(item, child, definitions)
    elif kind == 'bounded-array':
        need(type(value) is list and spec['minLength'] <= len(value) <= spec['maxLength'], 'bounded_array')
        for item in value:
            typed(item, spec['item'], definitions)
    elif kind == 'ordered-unique-subset':
        need(type(value) is list and len(value) == len(set(value))
             and value == [item for item in spec['values'] if item in value], 'ordered_subset')
    else:
        raise ValueError('unknown_schema_type')


def record(raw, name, definitions):
    value = decode(raw, 8192)
    typed(value, definitions[name], definitions)
    return value


GIT_STATE = None


def git(root, *args):
    # Git is a real direct child too: finite remaining phase/total time, passive
    # polling, raw logs and wait receipt. Never subprocess timeout-kill/retry.
    context = GIT_STATE
    need(context is not None, 'missing_git_clock_context')
    call_started = time.monotonic()
    deadline = min(context['overall'] + 1500, context['phaseDeadline'], call_started + 30)
    need(call_started < deadline, 'git_no_remaining_time')
    context['count'] += 1
    name = context['prefix'] + f"-{context['count']:04d}"
    paths = [context['out'] / (name + suffix) for suffix in ('.stdout.log', '.stderr.log')]
    argv = ['git', *args]
    receipt = {'name': name, 'argv': argv, 'exitCode': None, 'returned': False,
               'reaped': False, 'elapsedMs': None, 'error': 'none', 'logs': {}}
    try:
        with paths[0].open('xb') as stdout, paths[1].open('xb') as stderr:
            need(time.monotonic() < deadline, 'git_before_spawn_deadline')
            process = subprocess.Popen(argv, cwd=root, stdin=subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr, shell=False)
            while process.poll() is None:
                need(time.monotonic() < deadline, 'active_git_deadline')
                need(all(path.stat().st_size <= 4194304 for path in paths), 'active_git_log_cap')
                time.sleep(0.05)
            receipt['returned'] = True
            receipt['exitCode'] = process.wait()
            receipt['reaped'] = True
        need(time.monotonic() < deadline, 'returned_git_deadline')
        need(all(path.stat().st_size <= 4194304 for path in paths), 'returned_git_log_cap')
        data = [path.read_bytes() for path in paths]
        receipt['logs'] = {path.name: pin(raw) for path, raw in zip(paths, data)}
        need(receipt['exitCode'] == 0 and not data[1], 'git_actual_failure')
        need(time.monotonic() < deadline, 'git_postread_deadline')
        return data[0]
    except (OSError, ValueError) as error:
        receipt['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
        raise
    finally:
        receipt['elapsedMs'] = int((time.monotonic() - call_started) * 1000)
        if not receipt['reaped']:
            context['unreaped'] = True
        raw = compact(receipt) + b'\n'
        journal = context['out'] / (context['prefix'] + '-receipts.jsonl')
        need((journal.stat().st_size if journal.exists() else 0) + len(raw) <= 1048576,
             'git_journal_cap')
        with journal.open('ab') as output:
            output.write(raw)


def check_source(root, schema, identity=None):
    sha = os.environ['GITHUB_SHA']
    need(git(root, 'rev-parse', 'HEAD').decode().strip() == sha, 'head_changed')
    need(git(root, 'rev-list', '--parents', '-n', '1', 'HEAD').decode().split() == [sha, BASE], 'sole_parent')
    need(git(root, 'rev-parse', BASE + '^{tree}').decode().strip() == BASE_TREE, 'base_tree')
    tree = git(root, 'rev-parse', 'HEAD^{tree}').decode().strip()
    need(re.fullmatch('[0-9a-f]{40}', tree) is not None, 'checkout_tree')
    changes = [row.split('\t') for row in git(root, 'diff-tree', '--no-commit-id',
               '--name-status', '--no-renames', '-r', BASE, 'HEAD').decode().splitlines()]
    need(sorted(changes, key=lambda row: row[1]) == schema['changes'], 'exact_five_path_delta')
    observed = {}
    for name, expected in schema['sourcePins'].items():
        raw = (root / name).read_bytes()
        need(pin(raw) == expected, 'protected_source_bytes')
        observed[name] = pin(raw)
    need(pin((root / DATA).read_bytes()) == DATA_PIN, 'schema_bytes')
    for name in (SELF, WORKFLOW, DATA):
        observed[name] = pin((root / name).read_bytes())
    # Guard every tracked byte/mode against the immutable Git object, including
    # all untouched production paths. Only the four exact generated-schema
    # values below may change after the real successful Clippy build.
    entries = git(root, 'ls-tree', '-r', '-z', 'HEAD').split(b'\0')
    for entry in entries:
        if not entry:
            continue
        left, path = entry.split(b'\t', 1)
        mode, kind, oid = left.decode().split()
        name = path.decode('utf-8')
        need(mode in ('100644', '100755') and kind == 'blob', 'unsupported_tracked_mode')
        raw = (root / name).read_bytes()
        header = b'blob ' + str(len(raw)).encode('ascii') + b'\0'
        if name in schema['generatedSchemaGuards']:
            limits = schema['generatedSchemaGuards'][name]
            allowed = limits['afterAllowed'] if GIT_STATE['allowGenerated'] else [limits['before']]
            need(pin(raw) in allowed and mode == '100644', 'fixed_generated_schema_bytes')
        else:
            need(hashlib.sha1(header + raw).hexdigest() == oid, 'tracked_checkout_not_git_blob')
        if name in observed:
            need(mode == '100644', 'payload_mode')
    generated = {}
    for name in schema['generatedSchemaGuards']:
        raw = (root / name).read_bytes()
        actual = pin(raw)
        generated[name] = actual
        archive = GIT_STATE['out'] / ('generated-' + Path(name).stem + '-' + actual['sha256'] + '.json')
        if archive.exists():
            need(archive.read_bytes() == raw, 'generated_archive_changed')
        else:
            with archive.open('xb') as output:
                output.write(raw)
    snapshot = compact({'guard': GIT_STATE['sourceGuards'] + 1,
                        'phaseAllowsKnownGeneratedOutput': GIT_STATE['allowGenerated'],
                        'generatedObserved': generated, 'tree': tree}) + b'\n'
    journal = GIT_STATE['out'] / 'source-guards.jsonl'
    need((journal.stat().st_size if journal.exists() else 0) + len(snapshot) <= 1048576,
         'source_guard_journal_cap')
    with journal.open('ab') as output:
        output.write(snapshot)
    GIT_STATE['sourceGuards'] += 1
    if identity is not None:
        need(identity['payloads'] == observed and identity['tree'] == tree, 'immutable_identity_changed')
    return observed, tree


def host_guard():
    need(os.name == 'nt' and os.environ.get('GITHUB_ACTIONS') == 'true'
         and os.environ.get('RUNNER_ENVIRONMENT') == 'github-hosted'
         and os.environ.get('GITHUB_REF') == REF and os.environ.get('GITHUB_EVENT_NAME') == 'push'
         and os.environ.get('RUNNER_OS') == 'Windows' and os.environ.get('RUNNER_ARCH') == 'X64',
         'exact_hosted_push_required')
    need(re.fullmatch('[0-9]{1,20}', os.environ['GITHUB_RUN_ID']) is not None
         and re.fullmatch('[0-9]{1,6}', os.environ['GITHUB_RUN_ATTEMPT']) is not None
         and int(os.environ['GITHUB_RUN_ATTEMPT']) <= 2147483647
         and re.fullmatch('[0-9a-f]{40}', os.environ['GITHUB_SHA']) is not None, 'host_identity')
    # Only the workflow's byte-stable checkout settings may be inherited.
    need(os.environ.get('GIT_CONFIG_COUNT') == '2'
         and os.environ.get('GIT_CONFIG_KEY_0') == 'core.autocrlf'
         and os.environ.get('GIT_CONFIG_VALUE_0') == 'false'
         and os.environ.get('GIT_CONFIG_KEY_1') == 'core.eol'
         and os.environ.get('GIT_CONFIG_VALUE_1') == 'lf'
         and 'GIT_CONFIG_PARAMETERS' not in os.environ, 'git_checkout_settings')
    allowed = {'GIT_CONFIG_COUNT', 'GIT_CONFIG_KEY_0', 'GIT_CONFIG_VALUE_0',
               'GIT_CONFIG_KEY_1', 'GIT_CONFIG_VALUE_1'}
    need(not any(key.startswith(('GIT_CONFIG_KEY_', 'GIT_CONFIG_VALUE_')) and key not in allowed
                 for key in os.environ), 'extra_git_settings')
    need(not any(key in os.environ for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
         'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUST_TEST_THREADS', 'RUST_TEST_NOCAPTURE')),
         'unexpected_rust_runtime_override')


def elapsed_ms(start):
    return int((time.monotonic() - start) * 1000)


def guard_time(overall, stage_start, limit):
    need(time.monotonic() - overall < TOTAL_SECONDS and time.monotonic() - stage_start < limit,
         'deadline_uncovered')


def child(root, out, stage, name, argv, overall, identity, schema):
    guard_time(overall, stage['_start'], stage['limitSeconds'])
    check_source(root, schema, identity)
    guard_time(overall, stage['_start'], stage['limitSeconds'])
    item = {'name': name, 'argv': argv, 'exitCode': None, 'childReturned': False,
            'childReaped': False, 'elapsedMs': None, 'error': 'none'}
    stage['children'].append(item)
    stdout_path, stderr_path = (out / (name + suffix) for suffix in ('.stdout.log', '.stderr.log'))
    started = time.monotonic()
    try:
        with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
            guard_time(overall, stage['_start'], stage['limitSeconds'])
            env = dict(os.environ, CARGO_TERM_COLOR='never', RUST_BACKTRACE='0')
            # The raw GITHUB_RUN_ID is inherited unchanged by the actual test.
            process = subprocess.Popen(argv, cwd=root / 'windows/src-tauri',
                                       stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                       shell=False, env=env)
            while process.poll() is None:
                guard_time(overall, stage['_start'], stage['limitSeconds'])
                need(stdout_path.stat().st_size <= LOG_CAP and stderr_path.stat().st_size <= LOG_CAP,
                     'active_child_output_overflow')
                time.sleep(0.05)
            item['childReturned'] = True
            item['exitCode'] = process.wait()
            item['childReaped'] = True
        item['elapsedMs'] = elapsed_ms(started)
        guard_time(overall, stage['_start'], stage['limitSeconds'])
        need(stdout_path.stat().st_size <= LOG_CAP and stderr_path.stat().st_size <= LOG_CAP,
             'returned_child_output_overflow')
        item['logs'] = {path.name: pin(path.read_bytes()) for path in (stdout_path, stderr_path)}
        item['argvSha256'] = pin(compact(argv))['sha256']
        if name == 'strict-clippy' and item['exitCode'] == 0:
            GIT_STATE['allowGenerated'] = True
        check_source(root, schema, identity)
        guard_time(overall, stage['_start'], stage['limitSeconds'])
        return item, stdout_path.read_bytes(), stderr_path.read_bytes()
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        item['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
        raise
    finally:
        item['elapsedMs'] = elapsed_ms(started)
    # On timeout/overflow, never signal/retry/wait without a bound or hash active
    # logs. The stage stays unqualified/unreaped; the hosted job owns final cleanup.


def begin_stage(state, name, index):
    stage = {'name': name, 'limitSeconds': STAGE_LIMITS[index], 'children': [],
             'elapsedMs': None, 'complete': False, '_start': time.monotonic()}
    state['stages'].append(stage)
    GIT_STATE['phaseDeadline'] = stage['_start'] + stage['limitSeconds']
    return stage


def finish_stage(stage, overall):
    guard_time(overall, stage['_start'], stage['limitSeconds'])
    need(bool(stage['children']) and all(c['childReturned'] and c['childReaped'] for c in stage['children']),
         'stage_unreaped')
    stage['elapsedMs'] = elapsed_ms(stage['_start'])
    stage['complete'] = True


def good_q(value):
    need(all(item is True for item in value.values()), 'unqualified_client_current_time')


def eq_fields(value, expected, reason):
    need(all(value[key] == item and type(value[key]) is type(item) for key, item in expected.items()), reason)


def client_completed(slot, want):
    eq_fields(slot, {'socketCalls': 1, 'socketCreated': True, 'lingerSetCalls': 1,
              'lingerSetAck': True, 'lingerReadCalls': 1, 'lingerReadAck': True,
              'nonblockingCalls': 1, 'nonblockingAck': True, 'setupErrorReads': 0}, 'client_setup')
    c, s, e = (slot[key] for key in ('connect', 'select', 'soError'))
    need(c['calls'] == 1 and type(c['enteredSeq']) is int and type(c['returnedSeq']) is int
         and 0 < c['enteredSeq'] < c['returnedSeq'] and type(c['atMs']) is int
         and c['atMs'] < 45000, 'connect_single_qualified_return')
    good_q(c['q'])
    if c['disposition'] in ('ImmediateConnected', 'ImmediateRefused'):
        eq_fields(c, {'returned': 0 if want == 'Connected' else -1,
                  'lastError': None if want == 'Connected' else 10061,
                  'disposition': 'Immediate' + want}, 'immediate_connect_classification')
        for key, value in s.items():
            expected = ('Unobserved' if key == 'readyClass' else
                        {name: None for name in value} if key == 'q' else
                        0 if key in ('calls', 'noReady', 'errorReads') else None)
            need(value == expected and type(value) is type(expected), 'unexpected_immediate_select')
        for key, value in e.items():
            expected = ('Unobserved' if key == 'completion' else
                        {name: None for name in value} if key == 'q' else
                        False if key in ('queryConsumed', 'obligationCancelled') else
                        0 if key in ('calls', 'completionCount') else None)
            need(value == expected and type(value) is type(expected), 'unexpected_immediate_soerror')
        return c['returnedSeq'], c['atMs']
    eq_fields(c, {'disposition': 'Pending', 'returned': -1, 'lastError': 10035}, 'pending_connect_source')
    good_q(s['q'])
    good_q(e['q'])
    connected = want == 'Connected'
    eq_fields(s, {'readyClass': 'WriteOnly' if connected else 'ExceptOnly', 'lastReturn': 1,
              'lastError': None, 'errorReads': 0, 'writeCount': 1 if connected else 0,
              'exceptCount': 0 if connected else 1, 'writeCurrentMember': connected,
              'exceptCurrentMember': not connected}, 'select_readiness_not_unique')
    need(s['calls'] == s['noReady'] + 1 and s['calls'] > 0, 'select_opportunity_count')
    eq_fields(e, {'calls': 1, 'returned': 0, 'lastError': None, 'length': 4,
              'bufferValue': 0 if connected else 10061, 'valueAvailable': True,
              'queryConsumed': True, 'obligationCancelled': False, 'completionCount': 1,
              'completion': want}, 'soerror_not_actual_unique_completion')
    seqs = [c['returnedSeq'], s['readySeq'], e['enteredSeq'], e['returnedSeq']]
    need(all(type(v) is int for v in seqs) and all(a < b for a, b in zip(seqs, seqs[1:]))
         and e['fromReadySeq'] == s['readySeq'], 'completion_sequence_crossjoin')
    times = [c['atMs'], s['readyAtMs'], e['atMs']]
    need(all(type(v) is int for v in times) and times[0] <= times[1] <= times[2] < 45000,
         'completion_original_time')
    return e['returnedSeq'], e['atMs']


def qualified_raw(raw, run, definitions):
    r = record(raw, 'ControlledTransportV1', definitions)
    eq_fields(r, {'schema': 1, 'runId': run, 'caseId': CASE, 'variant': VARIANT,
              'result': 'legacyAdmissionRefused', 'failure': 'legacyAdmissionRefused',
              'complete': True, 'unknown': False, 'overflow': False}, 'not_exact_qualified_legacy_red')
    s, t, h, clock = (r[key] for key in ('server', 'transport', 'clientHarness', 'timing'))
    server_true = ('startupSucceeded', 'version22', 'exclusiveAck', 'listenerLingerOff',
                   'listenerNonblocking', 'boundEndpointMatched', 'listenSucceeded',
                   'acceptedLocalMatched', 'acceptedPeerMatched', 'streamLingerOff',
                   'streamNonblocking', 'finalInventoryObserved')
    need(all(s[key] is True for key in server_true), 'server_real_ack_or_inventory')
    eq_fields(s, {'startupAttempts': 1, 'socketCreates': 1, 'binds': 1, 'listens': 1, 'accepts': 1,
              'parsedRequests': 0, 'listenerCloseAttempts': 1, 'listenerCloseSuccesses': 1,
              'streamCloseAttempts': 1, 'streamCloseSuccesses': 1, 'cleanupAttempts': 1,
              'cleanupSuccesses': 1, 'requestBytes': 0, 'headerCount': 0, 'sentBytes': 0,
              'setupCalls': 9, 'acceptedSetupCalls': 4, 'socketsRemaining': 0,
              'startupRemaining': 0, 'serverThreads': 0, 'serverJoins': 0,
              'requestValid': None, 'responseComplete': None}, 'server_legacy_or_cleanup')
    eq_fields(t, {'version': 1, 'phase': 'Closed', 'acceptedTotal': 1, 'chosenSlot': None,
              'winnerBindings': 0, 'listenerRetirement': 'legacyFirstAccept', 'listenerRetired': True,
              'auxiliaryCount': 0, 'sealComplete': None, 'sendAttempts': 0, 'receivedBytes': 0,
              'firstChunkBytes': 0, 'socketPeak': 2, 'pendingAccept': False,
              'current': True, 'originalTimely': True}, 'transport_legacy_state')
    a_server, unused = t['slots']
    eq_fields(a_server, {'ordinal': 1, 'accepted': True, 'acceptedLocalMatched': True,
              'acceptedPeerMatched': True, 'lingerOff': True, 'nonblocking': True, 'setupCalls': 4,
              'recvBytes': 0, 'sendAttempts': 0, 'sentBytes': 0, 'eofCount': 0, 'chosen': False,
              'parsedRequests': 0, 'sealAttemptCount': 0, 'sealFinalCount': 0, 'sealClass': None,
              'closeAttempts': 1, 'closeSuccesses': 1, 'ownedAtFinal': False}, 'first_server_lease')
    nullable_slot = {'acceptedLocalMatched', 'acceptedPeerMatched', 'lingerOff', 'nonblocking', 'sealClass'}
    boolean_slot = {'accepted', 'chosen', 'ownedAtFinal'}
    for key, value in unused.items():
        expected = 2 if key == 'ordinal' else None if key in nullable_slot else False if key in boolean_slot else 0
        need(value == expected and type(value) is type(expected), 'unused_slot_not_zero_null')
    need(1 <= t['acceptAttempts'] <= 2048 and t['recvAttempts'] == a_server['recvAttempts']
         and a_server['nonprogressCount'] <= a_server['recvAttempts']
         and s['ioAttempts'] == t['acceptAttempts'] + t['recvAttempts'] + t['sendAttempts']
         and s['ioAttempts'] <= 2048, 'server_shared_io_counts')
    eq_fields(h, {'version': 1, 'tier': 'transportOnly'}, 'client_tier')
    eq_fields(h['startup'], {'calls': 1, 'returned': 0, 'version': 514, 'versionMatched': True,
              'succeeded': True, 'referenceOwned': False}, 'independent_client_startup')
    eq_fields(h['cleanup'], {'calls': 1, 'returned': 0, 'lastError': None, 'qualified': True,
              'referenceRemaining': 0, 'closeErrorReads': 0, 'cleanupErrorReads': 0,
              'eligibleBefore': True, 'noClientUseAfterAttempt': True}, 'independent_client_cleanup')
    eq_fields(h['ownership'], {'serverPeak': 2, 'clientPeak': 2, 'combinedPeak': 3,
              'remainingServerSockets': 0, 'remainingClientSockets': 0, 'inFlightClientActions': 0,
              'completionObligations': 0, 'harnessReferenceHeldAtServerCleanup': True,
              'noAmbientStartupReliance': True}, 'actual_transition_inventory')
    a, b = h['slots']
    need(a['ordinal'] == 1 and b['ordinal'] == 2, 'client_ordinals')
    a_seq, a_time = client_completed(a, 'Connected')
    _, b_time = client_completed(b, 'Refused')
    need(a_seq < b['connect']['enteredSeq'] and a_time <= b['connect']['atMs'], 'a_connected_before_b')
    for slot in (a, b):
        eq_fields(slot['data'], {'sendCalls': 0, 'recvCalls': 0, 'sendErrorReads': 0,
                  'recvErrorReads': 0, 'sentBytes': 0, 'receivedBytes': 0, 'requestMatched': None,
                  'responseMatched': None, 'originalTimely': None}, 'legacy_clients_must_have_no_application_io')
        eq_fields(slot['close'], {'calls': 1, 'returned': 0, 'lastError': None,
                  'qualified': True, 'ownedAtFinal': False, 'inFlight': False}, 'each_actual_client_close')
    setup_primary = sum(c['socketCalls'] + c['lingerSetCalls'] + c['lingerReadCalls']
                        + c['nonblockingCalls'] for c in (a, b))
    io_primary = sum(c['connect']['calls'] + c['select']['calls'] + c['soError']['calls']
                     + c['data']['sendCalls'] + c['data']['recvCalls'] for c in (a, b))
    io_errors = sum(int(c['connect']['returned'] == -1 and c['connect']['lastError'] is not None)
                    + c['select']['errorReads']
                    + int(c['soError']['returned'] == -1 and c['soError']['lastError'] is not None)
                    + c['data']['sendErrorReads'] + c['data']['recvErrorReads'] for c in (a, b))
    eq_fields(h['budgets'], {'setupCalls': setup_primary, 'setupErrorReads': 0,
              'ioCalls': io_primary + io_errors, 'ioErrorReads': io_errors,
              'primaryOperations': setup_primary + io_primary, 'spacingQualified': True,
              'originalClockMatched': True, 'overflow': False}, 'actual_client_budget_equations')
    need(h['budgets']['setupCalls'] <= 64 and h['budgets']['ioCalls'] <= 2048, 'client_caps')
    need(all(clock[key] is True for key in ('originalClockMatched', 'closeImmediatelyAfterTarget',
         'firstCloseAdopted', 'withinCloseDeadline')), 'clock_or_close_origin')
    target, close, end, cleanup = (clock[key] for key in
                                 ('targetAtMs', 'firstCloseAtMs', 'finalAtMs', 'cleanupElapsedMs'))
    need(all(type(v) is int for v in (target, close, end, cleanup))
         and b_time <= target < 45000 and target <= close <= end < 75000
         and cleanup < 10000 and end - close in (cleanup, cleanup + 1), 'original_45_10_75_time')
    return r


def rename_record(line, arm):
    # This existing whole-library producer intentionally bypasses libtest capture.
    # Preserve the raw stderr; validate the source's closed Record/ApiResult shape,
    # not one historical environment's error code as a new product expectation.
    prefix = 'Code0NativeRenameProbe '
    need(line.startswith(prefix), 'rename_record_prefix')
    value = decode(line[len(prefix):].encode('ascii'), 2048)
    keys = {'schema', 'arm', 'phase', 'result', 'source_identity_stable', 'root_identity_stable',
            'authority_valid_after', 'original_parent_target_matches', 'unrelated_cwd_target_matches', 'complete'}
    need(type(value) is dict and set(value) == keys, 'rename_closed_keys')
    eq_fields(value, {'schema': 1, 'arm': arm, 'complete': True,
              'phase': 'NativeRename' if arm in 'CD' else 'Win32Rename',
              'source_identity_stable': 'Yes', 'root_identity_stable': 'Yes',
              'authority_valid_after': 'Yes'}, 'rename_source_valid_complete')
    need(all(value[key] in ('Yes', 'No', 'Unknown') for key in
         ('original_parent_target_matches', 'unrelated_cwd_target_matches')), 'rename_observation')
    result = value['result']
    need(type(result) is dict, 'rename_result')
    if arm in 'CD':
        need(set(result) == {'api', 'status', 'io_status'} and result['api'] == 'Native'
             and type(result['status']) is int and -2147483648 <= result['status'] <= 0, 'native_rename_status')
        io = result['io_status']
        need(io == 'Unknown' or (type(io) is dict and set(io) == {'Status'}
             and type(io['Status']) is int and -2147483648 <= io['Status'] <= 0), 'native_rename_iostatus')
    else:
        need(set(result) == {'api', 'succeeded', 'error'} and result['api'] == 'Win32'
             and type(result['succeeded']) is bool, 'win32_rename_shape')
        error = result['error']
        need((result['succeeded'] and error == 'NotApplicable')
             or (not result['succeeded'] and type(error) is dict and set(error) == {'Code'}
                 and type(error['Code']) is int and 0 <= error['Code'] <= 4294967295), 'win32_rename_error')
    return value


def exact_suite(stdout_raw, stderr_raw, schema, run):
    stdout = stdout_raw.decode('utf-8').replace('\r\n', '\n')
    stderr = stderr_raw.decode('utf-8').replace('\r\n', '\n')
    need('\r' not in stdout and '\r' not in stderr and stdout.count('Code0Http401Transport ') == 1,
         'raw_framing')
    marker = re.search(r'Code0Http401Transport ([^\n]+)\n', stdout)
    need(marker is not None, 'missing_transport_raw_line')
    raw = marker.group(1).encode('ascii')
    qualified_raw(raw, run, schema['definitions'])
    # --nocapture prints the one marker after libtest's non-newline test prefix.
    # Removing exactly this one framed output restores its ordinary FAILED row.
    clean = stdout[:marker.start()] + stdout[marker.end():]
    rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', clean, re.MULTILINE)
    passed = sorted(name for name, result in rows if result == 'ok')
    failed = [name for name, result in rows if result == 'FAILED']
    need(len(rows) == 390 and len({name for name, _ in rows}) == 390
         and sorted(name for name, _ in rows) == schema['expectedTests']
         and passed == schema['oldTests'] and failed == [schema['newHttpTest']], 'complete_390_exact_results')
    summaries = re.findall(r'^test result: FAILED\. 389 passed; 1 failed; 0 ignored; 0 measured; '
                           r'0 filtered out; finished in [0-9]+(?:\.[0-9]+)?s$', clean, re.MULTILINE)
    expected = (['running 390 tests'] + [f'test {name} ... {result}' for name, result in rows]
                + ['failures:', 'failures:', '    ' + schema['newHttpTest']] + summaries)
    need(len(summaries) == 1 and [line for line in clean.splitlines() if line] == expected,
         'unexpected_stdout_or_failure_footer')
    need('panicked at' not in stdout, 'panic_in_stdout')
    # The binary is invoked directly. There is no Cargo prefix to discard or
    # silently reclassify. Existing five rename records and two exact panics are
    # the complete allowed stderr vocabulary, with all original bytes retained.
    rename_lines = re.findall(r'^Code0NativeRenameProbe [^\n]+\n', stderr, re.MULTILINE)
    need(len(rename_lines) == 5, 'five_existing_rename_arms_required')
    rename = [rename_record(line.rstrip('\n'), arm) for arm, line in zip('ABCDE', rename_lines)]
    residual = re.sub(r'^Code0NativeRenameProbe [^\n]+\n', '', stderr, flags=re.MULTILINE)
    worker = schema['workerPanic']
    http = schema['newHttpTest']
    wanted = {worker['thread']: (worker['path'], worker['line'], worker['column'], worker['message'] + '\n'),
              http: ('src/platform/windows/code0_webview/http401_native.rs', 6608, 9,
                     'assertion `left == right` failed: ' + schema['oracle']['assertion']
                     + '\n  left: Refused\n right: Connected\n')}
    normalized = residual.replace('\\', '/')
    headers = list(re.finditer(r"^thread '([^']+)' panicked at ([^\n]+):\n", normalized, re.MULTILINE))
    need(len(headers) == 2 and normalized.count('panicked at') == 2
         and {h.group(1) for h in headers} == set(wanted)
         and normalized[:headers[0].start()].strip() == '' and normalized.count(NOTE) <= 1,
         'exact_panic_headers')
    for index, header in enumerate(headers):
        name, location = header.groups()
        path, line, column, message = wanted[name]
        suffix = f'{path}:{line}:{column}'
        need(location == suffix or location.endswith('/' + suffix), 'exact_panic_location')
        end = headers[index + 1].start() if index + 1 < len(headers) else len(normalized)
        body = normalized[header.end():end]
        need(re.fullmatch(re.escape(message) + '(?:' + re.escape(NOTE) + ')?\n*', body) is not None,
             'exact_panic_semantics')
    need(worker['test'] in passed, 'intentional_worker_negative_control_must_pass')
    return raw, rename


def compiled_binary(stdout, root):
    candidates = []
    for line in stdout.splitlines():
        value = json.loads(line.decode('utf-8'), object_pairs_hook=unique,
                           parse_float=bad_number, parse_constant=bad_number)
        if value.get('reason') == 'compiler-artifact' and value.get('executable') is not None:
            if (value.get('target', {}).get('name') == 'ai_token_meter_windows'
                    and value.get('profile', {}).get('test') is True
                    and 'lib' in value.get('target', {}).get('kind', [])):
                need(value.get('features') == ['code0-native-fixture'], 'compiled_features')
                candidates.append(Path(value['executable']))
        if value.get('reason') == 'compiler-message':
            need(value.get('message', {}).get('level') not in ('error', 'warning'), 'no_run_diagnostic')
    need(len(candidates) == 1, 'exact_one_compiled_libtest')
    path = candidates[0].resolve(strict=True)
    deps = (root / 'windows/src-tauri/target/debug/deps').resolve(strict=True)
    need(path.parent == deps and re.fullmatch('ai_token_meter_windows-[0-9a-f]+\\.exe', path.name)
         is not None and path.is_file(), 'owned_compiled_libtest_path')
    return path


def gate_log(name, item):
    stdout = item['logs'][item['name'] + '.stdout.log']
    stderr = item['logs'][item['name'] + '.stderr.log']
    return {'name': name, 'exitCode': item['exitCode'], 'returned': item['childReturned'],
            'reaped': item['childReaped'], 'elapsedMs': item['elapsedMs'],
            'stdoutBytes': stdout['bytes'], 'stdoutSha256': stdout['sha256'],
            'stderrBytes': stderr['bytes'], 'stderrSha256': stderr['sha256'],
            'argvSha256': item['argvSha256']}


def main():
    global GIT_STATE
    host_guard()
    root = Path(os.environ['GITHUB_WORKSPACE']).resolve(strict=True)
    need(Path.cwd().resolve() == root and len(sys.argv) == 1
         and Path(__file__).resolve() == (root / SELF).resolve(), 'collector_invocation')
    schema_raw = (root / DATA).read_bytes()
    need(pin(schema_raw) == DATA_PIN, 'frozen_schema_data')
    schema = decode(schema_raw, 262144)
    definitions = schema['definitions']
    run, attempt = os.environ['GITHUB_RUN_ID'], int(os.environ['GITHUB_RUN_ATTEMPT'])
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-http401-transport-red-{run}-{attempt}'
    identity_raw = (out / 'identity.json').read_bytes()
    identity = decode(identity_raw, 16384)
    need(identity['runId'] == run and identity['attempt'] == attempt
         and identity['checkout'] == os.environ['GITHUB_SHA'] and identity['parent'] == BASE
         and identity['baseTree'] == BASE_TREE and identity['ref'] == REF
         and identity['event'] == 'push', 'preflight_identity')
    state = {'schema': 1, 'runId': run, 'attempt': attempt, 'checkout': identity['checkout'],
             'identityPin': pin(identity_raw), 'result': 'uncoveredFailure', 'error': 'none',
             'stages': [], 'allDirectChildrenReaped': False, 'elapsedMs': None,
             'qualifiedRecords': {}, 'wholeLibrary': None}
    started = time.monotonic()
    GIT_STATE = {'out': out, 'overall': started, 'phaseDeadline': started + 30,
                 'prefix': 'git', 'count': 0, 'unreaped': False,
                 'allowGenerated': False, 'sourceGuards': 0}
    binary = None
    try:
        stage = begin_stage(state, 'rust-and-format', 0)
        check_source(root, schema, identity)
        item, so, se = child(root, out, stage, 'rust-version', ['rustc', '-Vv'], started, identity, schema)
        need(item['exitCode'] == 0 and so.startswith(b'rustc 1.88.0 (')
             and b'host: x86_64-pc-windows-msvc' in so and b'panicked at' not in so and not se and len(so) <= 8192, 'rust_version_gate')
        fmt, so, se = child(root, out, stage, 'format', ['cargo', 'fmt', '--check'], started, identity, schema)
        need(fmt['exitCode'] == 0 and not so and not se, 'format_gate')
        finish_stage(stage, started)
        for index, name, argv, prefix in (
                (1, 'cargo-version', ['cargo', '-V'], b'cargo 1.88.0 ('),
                (2, 'clippy-version', ['cargo', 'clippy', '-V'], b'clippy 0.1.88 (')):
            stage = begin_stage(state, name, index)
            item, so, se = child(root, out, stage, name, argv, started, identity, schema)
            need(item['exitCode'] == 0 and so.startswith(prefix) and b'panicked at' not in so and len(so) <= 8192 and not se,
                 'version_gate')
            finish_stage(stage, started)
        stage = begin_stage(state, 'strict-clippy', 3)
        lint, so, se = child(root, out, stage, 'strict-clippy',
            ['cargo', 'clippy', '--locked', '--all-targets', '--features', 'code0-native-fixture',
             '--', '-D', 'warnings'], started, identity, schema)
        need(lint['exitCode'] == 0 and b'panicked at' not in so + se, 'strict_all_targets_clippy_gate')
        finish_stage(stage, started)
        stage = begin_stage(state, 'compile-and-list', 4)
        norun, so, se = child(root, out, stage, 'compile-no-run',
            ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture',
             '--lib', '--no-run', '--message-format=json'], started, identity, schema)
        need(norun['exitCode'] == 0 and b'panicked at' not in so + se, 'no_run_gate')
        binary = compiled_binary(so, root)
        binary_pin = pin(binary.read_bytes())
        need(binary_pin['bytes'] > 0, 'empty_binary')
        listing, so, se = child(root, out, stage, 'compiled-inventory',
            [str(binary), '--list', '--format', 'terse'], started, identity, schema)
        need(listing['exitCode'] == 0 and not se, 'compiled_inventory_exit')
        listed = re.findall(r'^(\S+): test$', so.decode('utf-8').replace('\r\n', '\n'), re.MULTILINE)
        need(len(listed) == 390 and len(set(listed)) == 390 and sorted(listed) == schema['expectedTests']
             and so.decode('utf-8').replace('\r\n', '\n').splitlines() == [n + ': test' for n in listed],
             'compiled_full_leafset_not_exact_390')
        inventory_raw = emit(out, 'compiled-inventory.json', sorted(listed), 131072)
        need(pin(binary.read_bytes()) == binary_pin, 'binary_changed_during_inventory')
        finish_stage(stage, started)
        sha, tree = identity['checkout'], identity['tree']
        source = {'schema': 1, 'runId': run, 'caseId': CASE, 'variant': VARIANT, 'commit': sha,
                  'tree': tree, 'parents': [BASE], 'candidate': sha, 'candidateTree': tree,
                  'candidateParents': [BASE], 'expectedRef': REF, 'event': 'push', 'runAttempt': attempt,
                  'toolchain': '1.88.0', 'target': 'x86_64-pc-windows-msvc',
                  'binaryBytes': binary_pin['bytes'], 'binarySha256': binary_pin['sha256'],
                  'reviewSha256': schema['sourceReviewSha256'], 'files': [],
                  'transportCore': {}, 'inventorySha256': pin(inventory_raw)['sha256'],
                  'totalTests': 390, 'expectedPasses': 389, 'expectedSemanticFailures': 1,
                  'oracle': schema['oracle']}
        file_names = [definitions[f'SourceIdentityV1_files_slot{n}']['fields']['path']['values'][0]
                      for n in range(1, 6)]
        source['files'] = [dict(path=name, **schema['sourcePins'][name]) for name in file_names]
        core = 'windows/src-tauri/src/platform/windows/code0_webview/http401_transport.rs'
        source['transportCore'] = dict(path=core, **schema['sourcePins'][core])
        typed(source, definitions['SourceIdentityV1'], definitions)
        source_raw = emit(out, 'source.json', source, 8192)
        # Seal these exact records before starting the selected complete child.
        check_source(root, schema, identity)
        need(pin(binary.read_bytes()) == binary_pin, 'binary_changed_before_library')
        stage = begin_stage(state, 'complete-library', 5)
        suite, so, se = child(root, out, stage, 'complete-library',
            [str(binary), '--nocapture', '--test-threads=1'], started, identity, schema)
        exit_record = {'schema': 1, 'runId': run, 'exitCode': suite['exitCode']}
        typed(exit_record, definitions['ExitV1'], definitions)
        exit_raw = emit(out, 'exit.json', exit_record, 8192)
        need(suite['exitCode'] == 101 and suite['childReturned'] and suite['childReaped']
             and suite['elapsedMs'] < 75000, 'sole_qualified_red_exit_or_child_bound')
        raw, rename = exact_suite(so, se, schema, run)
        emit(out, 'existing-rename-records.json', rename, 16384)
        # Save the verbatim embedded JSON, with no producer field edits/newline.
        with (out / 'controlled-raw.json').open('xb') as target:
            target.write(raw)
        need((out / 'source.json').read_bytes() == source_raw
             and (out / 'compiled-inventory.json').read_bytes() == inventory_raw
             and (out / 'exit.json').read_bytes() == exit_raw, 'sealed_records_changed')
        check_source(root, schema, identity)
        need(pin(binary.read_bytes()) == binary_pin, 'binary_changed_after_library')
        finish_stage(stage, started)
        need(len(state['stages']) == 6 and all(s['complete'] for s in state['stages']), 'six_stages_required')
        collector = {'schema': 1, 'runId': run, 'caseId': CASE, 'variant': VARIANT,
                     'sourceSha256': pin(source_raw)['sha256'], 'rawBytes': len(raw),
                     'rawSha256': pin(raw)['sha256'], 'exitBytes': len(exit_raw),
                     'exitSha256': pin(exit_raw)['sha256'], 'stdoutBytes': len(so),
                     'stdoutSha256': pin(so)['sha256'], 'stderrBytes': len(se),
                     'stderrSha256': pin(se)['sha256'], 'childReturned': suite['childReturned'],
                     'childReaped': suite['childReaped'], 'exitCode': suite['exitCode'],
                     'elapsedMs': suite['elapsedMs'], 'oldSuiteMatched': True, 'oracleMatched': True,
                     'gates': {'formatExit': fmt['exitCode'], 'strictClippyExit': lint['exitCode'],
                               'noRunExit': norun['exitCode'], 'completeSuiteExit': suite['exitCode']},
                     'sourceUnchangedBeforeAfter': True, 'binaryUnchangedBeforeAfter': True,
                     'collectorExit': 0, 'qualification': 'acceptedLegacyTransportRED',
                     'gateLogs': [gate_log(n, c) for n, c in
                                  [('format', fmt), ('strictClippy', lint), ('noRun', norun), ('completeSuite', suite)]]}
        typed(collector, definitions['CollectorV1'], definitions)
        # This local zero is a shape check, not an observed collector exit.
        # Only the workflow parent can fill collectorExit after this process
        # actually returns. No final CollectorV1 is emitted by the child.
        del collector['collectorExit']
        collector_input = emit(out, 'collector-input.json', collector, 8192)
        state['qualifiedRecords'] = {name: pin(value) for name, value in
                                    [('source.json', source_raw), ('controlled-raw.json', raw),
                                     ('exit.json', exit_raw), ('collector-input.json', collector_input)]}
        state['wholeLibrary'] = {'compiledListCount': 390, 'passed': 389, 'failed': 1,
                                 'inventoryPin': pin(inventory_raw), 'binaryPin': binary_pin,
                                 'workerNegativeControl': schema['workerPanic'],
                                 'existingRenameArms': 5, 'soleFailedTest': schema['newHttpTest']}
        need(time.monotonic() - started < TOTAL_SECONDS, 'final_total_deadline')
        state['result'] = 'qualifiedLegacyEvidencePendingParentSeal'
    except (OSError, ValueError, UnicodeError, KeyError, subprocess.CalledProcessError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        for stage in state['stages']:
            stage_start = stage.pop('_start')
            if not stage['complete']:
                stage['elapsedMs'] = elapsed_ms(stage_start)
        state['allDirectChildrenReaped'] = (bool(state['stages']) and not GIT_STATE['unreaped']
            and all(c['childReaped'] for s in state['stages'] for c in s['children']))
        for name in ('git-receipts.jsonl', 'source-guards.jsonl'):
            path = out / name
            if path.exists():
                state[name] = pin(path.read_bytes())
        state['gitCallCount'] = GIT_STATE['count']
        state['sourceGuardCount'] = GIT_STATE['sourceGuards']
        state['elapsedMs'] = elapsed_ms(started)
        emit(out, 'summary.json', state, 65536)
    need(state['result'] == 'qualifiedLegacyEvidencePendingParentSeal' and state['allDirectChildrenReaped']
         and time.monotonic() - started < TOTAL_SECONDS, 'HTTP_transport_RED_not_established')
    print('Exact390 full library: old389 PASS and sole source-bound qualified HTTP RED; no native Basic/Session or production shutdown claim.')


if __name__ == '__main__':
    main()
