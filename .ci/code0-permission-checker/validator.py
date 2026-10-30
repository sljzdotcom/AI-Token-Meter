"""Standalone schema14 evidence checker. SOURCE ONLY; execution UNRUN.

Only the permission schema is understood. Fresh source/attempt and truthful
native production are orchestration/review obligations, not proved by JSON.
"""
import argparse
import json
import re
from pathlib import Path

SELECTOR = 'refusal-notification-owned-https'
U64_MAX = 18446744073709551615
ALL_COVERAGE = 16777215
HOOKS = ('topNavigation', 'childNavigation', 'externalProtocol', 'newWindow',
         'download', 'scriptDialog', 'permission', 'basicAuthentication',
         'clientCertificate', 'tlsError')
PHASES = ('Registering', 'Enabling', 'Navigating', 'Armed', 'Issued', 'Closing',
          'Cutoff', 'Released')
PROBLEMS = ('NotReady', 'Shape', 'Identity', 'Affinity', 'Clock', 'Deadline',
            'CloseDeadline', 'Coverage', 'Phase', 'Replay', 'Root', 'Context',
            'Lifecycle', 'Budget', 'Native', 'Result', 'Released', 'Overflow')
CUTOFFS = ('RequiredRemovalAttempt', 'CoverageLost', 'ControlsDisposal',
           'StopEntry', 'ControllerCloseEntry', 'AffinityLost')
RESULTS = ('denied', 'precondition', 'default', 'granted', 'documentChanged',
           'invalid', 'malformed')
FAILURES = ('none', 'notCompleted', 'watchdogTimeout', 'fixtureSetupFailed',
            'inheritedWebview2Override', 'uiAffinityFailed', 'bareWindowViolation',
            'permissionSetupRejected', 'permissionPreflightFailed',
            'permissionNativeTargetMissing', 'permissionNativeMetadataFailed',
            'permissionNativeActionFailed', 'permissionNativeStateMismatch',
            'permissionEffectMissing', 'permissionEffectLate',
            'permissionIdentityChanged', 'permissionProtocolRejected',
            'permissionDeadlineExceeded', 'permissionCleanupFailed',
            'permissionOverflow', 'permissionUnknown')
INSTALL_COUNTS = ('submissions', 'barrierReceipts', 'firstLoadReceipts', 'documentReceipts')
INSTALL_FLAGS = ('submissionAccepted', 'installRejected', 'scriptAcknowledged',
                 'cdpAcknowledged', 'sourceMatched', 'syntheticResponseApplied',
                 'canonicalReadyControl', 'ackBeforeFirstLoad')
LIFE_TRUE = ('staMain', 'callbackAffinity', 'noInheritedOverride', 'engineReady',
             'bareHiddenUnfocused', 'constructionObserved', 'profileIsolated',
             'destroyed', 'ownedReferencesReleased', 'runReturned', 'cleanupSucceeded')
SETUP_FLAGS = ('settingsAck', 'responderReady', 'receiversReady', 'domainsAck',
               'documentBound', 'lifecycleAssemblyComplete', 'blankBranchClosed',
               'observationCoverageIntactAtArm', 'originalDeadlineBound')
ONCE = ('navigation', 'mainResponses', 'permissionEntries', 'kindReads', 'gestureReads',
        'originReads', 'stateBeforeReads', 'denyAttempts', 'denyReturns', 'denySuccesses',
        'stateAfterReads', 'actionReceipts', 'evaluateSubmissions', 'evaluateCompletions',
        'effectReceipts', 'sessionPolicyFailures', 'finalReceipts')
COMMAND_TOTALS = ('commandIssued', 'commandReturned', 'commandCompleted', 'commandConsumed')
NATIVE = ('senderMatched', 'requestInstallationMatched', 'kindNotifications', 'nonUser',
          'originMatched', 'stateBeforeDefault', 'denySucceeded', 'stateAfterDeny',
          'originalTimely', 'postDropMatched', 'enteredBeforeObservationCutoff',
          'capturedBeforeObservationCutoff', 'qualifiedBeforeObservationCutoff')
EFFECT = ('successHresult', 'closedDeniedResult', 'sameTicket', 'sameFrameLoaderContext',
          'sourceMatched', 'invokedAfterNativeAction', 'invokedBeforeStop',
          'capturedBeforeStop', 'qualifiedBeforeStop', 'enteredBeforeObservationCutoff',
          'capturedBeforeObservationCutoff', 'qualifiedBeforeObservationCutoff',
          'originalTimely', 'closeTimely', 'epochUnchangedThroughQualification')
ORDERING = ('permanentRevocationAtRefusal', 'firstCloseAtRefusal', 'noActionReauthorized',
            'noWaitForEffect', 'actualPolicyFailureAfterAction', 'cutoffLatched',
            'cutoffBeforeAnyRequiredRemoval', 'qualifiedObservationsAreHistorical',
            'physicalCloseObserved')
DRAIN = ('obligationsEmpty', 'handlersEmpty', 'textEmpty', 'receiverTokensRemoved',
         'resourceAndFilterRemoved', 'controlsEmpty')
COVER_COUNTERS = ('addAttempts', 'addAcks', 'removeAttempts', 'removeAcks')
COVER_RETURNS = ('addReturns', 'removeReturns')
COVER_MASKS = ('requiredMask', 'addedMask', 'removeAttemptMask', 'removedMask',
               'uncertainMask', 'ownedMask')
COMMAND_COUNTERS = ('issued', 'returned', 'completed', 'consumed', 'handlerDrops')
COMMAND_FLAGS = ('submissionSucceeded', 'completionSucceeded', 'qualified')
TIMES = ('nativeEnteredAtMs', 'nativeCapturedAtMs', 'nativeQualifiedAtMs',
         'effectEnteredAtMs', 'effectCapturedAtMs', 'effectQualifiedAtMs',
         'firstCloseAtMs', 'observationCutoffAtMs', 'stopEnteredAtMs',
         'controllerCloseEnteredAtMs', 'releasedAtMs', 'finalAtMs', 'cleanupElapsedMs')
TIME_FLAGS = ('finalWithinCloseDeadline', 'sameOriginalClock', 'firstCloseAdopted')
CLEANUP_COUNTS = ('browserProcessExits', 'windowDestroyed', 'wakeJoins', 'tempDirCloseAttempts')
CLEANUP_FLAGS = ('releasedRefZero', 'wakeShutdownIssued', 'wakeJoinSucceeded',
                 'tempDirCloseSucceeded', 'tempDirResultDropped')


class ValidationError(ValueError):
    """Closed evidence validation failure, without echoing supplied content."""


def require(condition, reason):
    if not condition:
        raise ValidationError(reason)


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate decoded member')
        result[key] = value
    return result


def noninteger(_):
    raise ValidationError('noninteger JSON number')


def decode(raw):
    require(type(raw) is bytes and len(raw) <= 8192 and raw.isascii(),
            'oversize, nonbytes or non-ASCII evidence')
    # The closed schema's deepest container is root/permission/group/array.
    # Lexical scan avoids asking json.loads to recurse through untrusted depth.
    depth = 0
    quoted = False
    escaped = False
    for byte in raw:
        if quoted:
            if escaped:
                escaped = False
            elif byte == 92:
                escaped = True
            elif byte == 34:
                quoted = False
        elif byte == 34:
            quoted = True
        elif byte in (91, 123):
            depth += 1
            require(depth <= 4, 'evidence nesting exceeds closed profile')
        elif byte in (93, 125):
            depth -= 1
            require(depth >= 0, 'unbalanced evidence')
    try:
        return json.loads(raw.decode('utf-8'), object_pairs_hook=unique,
                          parse_float=noninteger, parse_constant=noninteger)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise ValidationError('invalid closed JSON evidence') from error


def bounded_read(stream):
    return decode(stream.read(8193))


def read(path):
    with Path(path).open('rb') as stream:
        return bounded_read(stream)


def fields(keys, spec):
    return dict.fromkeys(keys, spec)


def nullable(spec):
    return ('nullable', spec)


B = ('bool',)
Q = nullable(B)
C = ('counter', 1)
U8 = ('integer', 0, 255)
U16 = ('integer', 0, 65535)
M = ('integer', 0, ALL_COVERAGE)
S16 = ('integer', 0, 65535)
T = nullable(('integer', 0, U64_MAX))
COMMAND_VALUES = ('tuple', (
    ('enum', (None, 'emptyAck', 'malformed')),
    ('enum', (None, 'emptyAck', 'malformed')),
    ('enum', (None, 'frameTree', 'malformed')),
    ('enum', (None,) + RESULTS),
))
SUMMARY = {
    'schema': ('enum', (14,)), 'runId': ('string', r'[A-Za-z0-9_-]{1,64}'),
    'requiredCase': ('enum', (SELECTOR,)), 'selectedCases': U8, 'completedCases': U8,
    'pass': B, 'failure': ('enum', FAILURES),
    'install': {**fields(INSTALL_COUNTS, U8), **fields(INSTALL_FLAGS, B)},
    'lifecycle': {**fields(LIFE_TRUE, B), 'bareWindowViolation': B,
                  'runtimeVersion': ('string', r'(?:[0-9.]{1,128})?')},
    'nativeRefusal': {
        'receipts': U8, **fields(('nativeInstanceValidated', 'sourceBoundRegistry',
                                 'navigationNotIssued', 'loadRejected'), B),
        'requiredMask': U16, 'installedMask': U16, 'notInstalledHooks': ('hooks',),
    },
    'permission': {
        'version': ('enum', (1,)), 'selector': ('enum', (SELECTOR,)),
        'phase': ('enum', PHASES + ('Closed',)), 'unknown': B, 'overflow': B,
        'setup': {'hooks1023': U16, **fields(SETUP_FLAGS, Q)},
        'counts': {**fields(ONCE, C), 'unexpectedRequests': ('counter', 16),
                   **fields(('receiverAdds', 'receiverRemoves'), ('counter', 10)),
                   **fields(COMMAND_TOTALS, ('counter', 4))},
        'native': fields(NATIVE, Q), 'effect': fields(EFFECT, Q),
        'ordering': fields(ORDERING, Q), 'drain': fields(DRAIN, Q),
        'coverage': {**fields(COVER_MASKS, M),
                     **fields(COVER_COUNTERS, ('array', 24, C)),
                     **fields(COVER_RETURNS, ('array', 24, Q))},
        'commands': {**fields(COMMAND_COUNTERS, ('array', 4, C)),
                     **fields(COMMAND_FLAGS, ('array', 4, Q)),
                     'values': COMMAND_VALUES,
                     **fields(('obligations', 'handlers'), ('inventory', 4)),
                     'inFlight': ('inventory', 1)},
        'observations': {
            'blankBranch': ('enum', (None, 'ZeroObserved', 'Observed', 'ObservedInitialStopOnly')),
            **fields(('bootstrapDeliveries', 'disposalDeliveries', 'deliveries',
                      'postArmPreCutoff'), S16),
            'byReceiver': ('array', 10, S16), 'counterOverflow': B,
        },
        'inventory': {
            **fields(('textBytes', 'textPeakBytes'), ('inventory', 4096)),
            'textReserveFailures': C, 'textAccountingComplete': Q,
            **fields(('activeGuards', 'aggregateHandlers'), ('inventory', 65535)),
            'newEventHandlers': ('inventory', 12), 'commandHandlers': ('inventory', 4),
            'handlerAccountingComplete': Q,
            **fields(('recordSlots', 'recordSlotsPeak'), ('inventory', 3)),
            **fields(('completionSlot', 'documentRecord', 'nativeRecord'), ('inventory', 1)),
        },
        'timing': {**fields(TIMES, T), **fields(TIME_FLAGS, Q)},
        'status': {
            'protocolPhase': ('enum', (None,) + PHASES),
            'protocolFailure': ('enum', (None,) + PROBLEMS),
            'observationEpoch': ('enum', (None, 0, 1)),
            'cutoffReason': ('enum', (None,) + CUTOFFS),
            **fields(('nativeQualified', 'effectQualified'), Q),
            'effectResult': ('enum', (None,) + RESULTS),
            'sessionReason': ('enum', (None, 'PolicyRejected', 'other')),
            **fields(('navigationSucceeded', 'mainResponseSucceeded'), Q),
            'terminalResult': ('enum', (None, 'succeeded', 'failed')),
        },
        'cleanup': {**fields(CLEANUP_COUNTS, C), **fields(CLEANUP_FLAGS, Q)},
    },
}
EXIT = {'schema': ('enum', (1,)), 'runId': ('string', r'[A-Za-z0-9_-]{1,64}'),
        'exitCode': ('integer', -2147483648, 2147483647)}


def check(value, spec):
    if type(spec) is dict:
        require(type(value) is dict and set(value) == set(spec), 'wrong closed object keys/type')
        for key, child in spec.items():
            check(value[key], child)
        return
    kind = spec[0]
    if kind == 'nullable':
        if value is not None:
            check(value, spec[1])
    elif kind == 'bool':
        require(type(value) is bool, 'wrong bool type')
    elif kind == 'integer':
        require(type(value) is int and spec[1] <= value <= spec[2], 'wrong integer range/type')
    elif kind in ('counter', 'inventory'):
        require((kind == 'inventory' and value is None)
                or (type(value) is int and 0 <= value <= spec[1] + 1),
                'wrong counter/inventory range/type')
    elif kind == 'enum':
        require(any(type(value) is type(item) and value == item for item in spec[1]),
                'unknown enum or wrong enum type')
    elif kind == 'string':
        require(type(value) is str and re.fullmatch(spec[1], value) is not None,
                'wrong bounded string')
    elif kind in ('array', 'tuple'):
        length = spec[1] if kind == 'array' else len(spec[1])
        require(type(value) is list and len(value) == length, 'wrong fixed array length/type')
        for index, item in enumerate(value):
            check(item, spec[2] if kind == 'array' else spec[1][index])
    elif kind == 'hooks':
        require(type(value) is list and len(value) <= 10, 'wrong hook inventory type/length')
        require(all(type(item) is str and item in HOOKS for item in value), 'unknown hook')
        indexes = [HOOKS.index(item) for item in value]
        require(indexes == sorted(set(indexes)), 'duplicate/unordered hook inventory')
    else:
        raise ValidationError('invalid internal schema definition')


def saturated(value, spec):
    if type(spec) is dict:
        return any(saturated(value[key], child) for key, child in spec.items())
    if spec[0] in ('counter', 'inventory'):
        return value is not None and value == spec[1] + 1
    if spec[0] == 'array':
        return any(saturated(item, spec[2]) for item in value)
    if spec[0] == 'tuple':
        return any(saturated(item, child) for item, child in zip(value, spec[1]))
    if spec[0] == 'nullable' and value is not None:
        return saturated(value, spec[1])
    return False


def counter_sum_matches(total, rows, cap):
    # A C=2 row is only a lower bound. Its actual count can exceed2; do not
    # demand equality between an aggregate and a sum of saturated lower bounds.
    lower = min(cap + 1, sum(rows))
    return total >= lower if 2 in rows else total == lower


def check_shape(data, exit_data, run_id):
    """Check closed bounded partial evidence, without accepting a failed case."""
    require(type(run_id) is str and re.fullmatch(r'[A-Za-z0-9_-]{1,64}', run_id) is not None,
            'invalid expected run identity')
    check(data, SUMMARY)
    check(exit_data, EXIT)
    require(data['runId'] == exit_data['runId'] == run_id, 'stale or mismatched run identity')
    p = data['permission']
    c, cov, cmd = p['counts'], p['coverage'], p['commands']
    obs, inv, status = p['observations'], p['inventory'], p['status']
    require(not (p['unknown'] or p['overflow']) or data['failure'] != 'none',
            'unknown/overflow with successful failure code')
    require(not saturated(p, SUMMARY['permission']) or p['overflow'],
            'saturated lower bound without overflow')
    require(not obs['counterOverflow'] or p['overflow'], 'delivery overflow not sticky')
    old = data['nativeRefusal']
    require((old['requiredMask'] | old['installedMask']) & ~1023 == 0,
            'unsupported old hook mask')
    missing = [name for i, name in enumerate(HOOKS)
               if old['requiredMask'] & (1 << i) and not old['installedMask'] & (1 << i)]
    require(old['notInstalledHooks'] == missing, 'old hook mask/list mismatch')
    require(cov['requiredMask'] == ALL_COVERAGE, 'wrong required coverage')
    require(cov['uncertainMask'] & ~cov['ownedMask'] == 0, 'uncertain ownership discarded')
    for row in range(24):
        bit = 1 << row
        for mask, array in (('addedMask', 'addAcks'), ('removeAttemptMask', 'removeAttempts'),
                            ('removedMask', 'removeAcks')):
            require(bool(cov[mask] & bit) == (cov[array][row] > 0), 'coverage mask/row mismatch')
        for prefix in ('add', 'remove'):
            if cov[prefix + 'Acks'][row] > 0:
                require(cov[prefix + 'Attempts'][row] > 0
                        and cov[prefix + 'Returns'][row] is True,
                        'coverage ACK without attempt/successful return')
    for name, array in (('receiverAdds', 'addAcks'), ('receiverRemoves', 'removeAcks')):
        require(counter_sum_matches(c[name], cov[array][10:20], 10), 'receiver totals mismatch')
    for name, array in zip(COMMAND_TOTALS, ('issued', 'returned', 'completed', 'consumed')):
        require(counter_sum_matches(c[name], cmd[array], 4), 'command totals mismatch')
    require(c['evaluateSubmissions'] == cmd['issued'][3]
            and c['evaluateCompletions'] == cmd['completed'][3], 'evaluate row mismatch')
    require(status['effectResult'] == cmd['values'][3], 'effect result rows disagree')
    require(cmd['handlers'] == inv['commandHandlers'], 'handler inventories disagree')
    if not obs['counterOverflow']:
        require(sum(obs['byReceiver']) == obs['deliveries'], 'delivery receiver sum mismatch')
        require(obs['deliveries'] == obs['bootstrapDeliveries'] + obs['disposalDeliveries']
                + obs['postArmPreCutoff'], 'delivery phase sum mismatch')
    if not p['overflow']:
        require(c['denySuccesses'] <= c['denyReturns'] <= c['denyAttempts'],
                'DENY return without attempt')
        for row in range(4):
            require(cmd['returned'][row] <= cmd['issued'][row]
                    and cmd['consumed'][row] <= cmd['completed'][row] <= cmd['issued'][row],
                    'completion disposition without actual issue/invocation')
        if cmd['obligations'] is not None:
            require(cmd['obligations'] == sum(cmd['issued']) - sum(cmd['consumed']),
                    'obligation erased without consumed invocation')
        slots = [inv[key] for key in ('completionSlot', 'documentRecord', 'nativeRecord')]
        if inv['recordSlots'] is not None and all(item is not None for item in slots):
            require(inv['recordSlots'] == sum(slots), 'record inventory sum mismatch')
        if inv['recordSlots'] is not None and inv['recordSlotsPeak'] is not None:
            require(inv['recordSlotsPeak'] >= inv['recordSlots'], 'record peak below current')
        if inv['textBytes'] is not None and inv['textPeakBytes'] is not None:
            require(inv['textPeakBytes'] >= inv['textBytes'], 'text peak below current')
    require(not data['pass'] or data['failure'] == 'none', 'passing failure code')
    require(data['pass'] == (status['terminalResult'] == 'succeeded'), 'terminal verdict mismatch')
    require(p['phase'] != 'Closed' or status['terminalResult'] is not None,
            'Closed without terminal disposition')


def validate(data, exit_data, run_id):
    """Accept only complete strict milestone B plus actual matching exit0."""
    check_shape(data, exit_data, run_id)
    require(data['pass'] is True and data['failure'] == 'none'
            and data['selectedCases'] == data['completedCases'] == 1
            and exit_data['exitCode'] == 0, 'not actual selected completed GREEN/exit0')
    install, life, old = data['install'], data['lifecycle'], data['nativeRefusal']
    require(all(install[key] == 0 for key in INSTALL_COUNTS)
            and all(install[key] is False for key in INSTALL_FLAGS), 'P1 fields changed meaning')
    require(all(life[key] is True for key in LIFE_TRUE)
            and life['bareWindowViolation'] is False and life['runtimeVersion'] != '',
            'missing shared actual lifecycle/runtime')
    require(old['receipts'] == 1 and old['nativeInstanceValidated'] is True
            and old['sourceBoundRegistry'] is True and old['navigationNotIssued'] is False
            and old['loadRejected'] is False and old['requiredMask'] == 1023
            and old['installedMask'] == 1023 and old['notInstalledHooks'] == [],
            'missing unchanged shared refusal gate')
    p = data['permission']
    require(p['phase'] == 'Closed' and p['unknown'] is False and p['overflow'] is False,
            'permission incomplete/unknown/overflow')
    require(p['setup']['hooks1023'] == 1023
            and all(p['setup'][key] is True for key in SETUP_FLAGS), 'missing real setup')
    counts = p['counts']
    require(all(counts[key] == 1 for key in ONCE)
            and counts['unexpectedRequests'] == 0
            and counts['receiverAdds'] == counts['receiverRemoves'] == 10
            and all(counts[key] == 4 for key in COMMAND_TOTALS), 'wrong actual receipt/action totals')
    for group, keys in (('native', NATIVE), ('effect', EFFECT), ('ordering', ORDERING), ('drain', DRAIN)):
        require(all(p[group][key] is True for key in keys), 'missing strict B boundary fact')
    cov = p['coverage']
    require(all(cov[key] == ALL_COVERAGE for key in COVER_MASKS[:4])
            and cov['uncertainMask'] == cov['ownedMask'] == 0
            and all(all(item == 1 for item in cov[key]) for key in COVER_COUNTERS)
            and all(all(item is True for item in cov[key]) for key in COVER_RETURNS),
            'required coverage not genuinely added/removed/drained')
    cmd = p['commands']
    require(all(all(item == 1 for item in cmd[key]) for key in COMMAND_COUNTERS)
            and all(all(item is True for item in cmd[key]) for key in COMMAND_FLAGS)
            and cmd['values'] == ['emptyAck', 'emptyAck', 'frameTree', 'denied']
            and cmd['obligations'] == cmd['handlers'] == cmd['inFlight'] == 0,
            'missing fixed command invocation/disposition/handler drain')
    obs = p['observations']
    require(obs['blankBranch'] in ('ZeroObserved', 'Observed', 'ObservedInitialStopOnly')
            and 4 <= obs['bootstrapDeliveries'] <= 16
            and obs['postArmPreCutoff'] == 0 and obs['counterOverflow'] is False,
            'missing closed blank/lifecycle assembly')
    inv = p['inventory']
    require(inv['textBytes'] == 0 and inv['textPeakBytes'] is not None
            and 1 <= inv['textPeakBytes'] <= 4096 and inv['textReserveFailures'] == 0
            and inv['textAccountingComplete'] is True
            and all(inv[key] == 0 for key in ('activeGuards', 'aggregateHandlers',
                                             'newEventHandlers', 'commandHandlers', 'completionSlot'))
            and inv['handlerAccountingComplete'] is True
            and inv['documentRecord'] in (0, 1) and inv['nativeRecord'] in (0, 1)
            and inv['recordSlots'] is not None and 0 <= inv['recordSlots'] <= 2
            and inv['recordSlotsPeak'] is not None and 2 <= inv['recordSlotsPeak'] <= 3,
            'missing concurrent text/real inventory/history-slot accounting')
    status = p['status']
    require(status['protocolPhase'] == 'Released' and status['protocolFailure'] is None
            and status['observationEpoch'] == 1
            and status['cutoffReason'] in ('RequiredRemovalAttempt', 'ControlsDisposal',
                                          'StopEntry', 'ControllerCloseEntry')
            and status['nativeQualified'] is True and status['effectQualified'] is True
            and status['effectResult'] == 'denied' and status['sessionReason'] == 'PolicyRejected'
            and status['navigationSucceeded'] is True and status['mainResponseSucceeded'] is True
            and status['terminalResult'] == 'succeeded', 'missing strict native/effect/Session status')
    cleanup = p['cleanup']
    require(all(cleanup[key] == 1 for key in CLEANUP_COUNTS)
            and all(cleanup[key] is True for key in CLEANUP_FLAGS), 'incomplete actual cleanup')
    timing = p['timing']
    require(all(timing[key] is not None for key in TIMES)
            and all(timing[key] is True for key in TIME_FLAGS), 'missing original precise timing')
    chain = ('nativeEnteredAtMs', 'nativeCapturedAtMs', 'nativeQualifiedAtMs',
             'effectEnteredAtMs', 'effectCapturedAtMs', 'effectQualifiedAtMs',
             'observationCutoffAtMs', 'stopEnteredAtMs', 'controllerCloseEnteredAtMs',
             'releasedAtMs', 'finalAtMs')
    require(all(timing[a] <= timing[b] for a, b in zip(chain, chain[1:])),
            'callback/capture/qualification/cutoff/physical cleanup order mismatch')
    require(all(timing[key] < 45000 for key in chain[:6])
            and timing['nativeEnteredAtMs'] <= timing['firstCloseAtMs'] <= timing['nativeCapturedAtMs']
            and timing['cleanupElapsedMs'] < 10000
            and timing['finalAtMs'] - timing['firstCloseAtMs']
            in (timing['cleanupElapsedMs'], timing['cleanupElapsedMs'] + 1),
            'invalid original45/first-close10/rounding evidence')
    # Precise flags and historical native/effect boundary predicates above are
    # independent mandatory facts; rounded milliseconds do not authorize expiry.


def main():
    parser = argparse.ArgumentParser(description='Accept only schema14 strict permission B evidence')
    parser.add_argument('--summary', required=True)
    parser.add_argument('--exit', dest='exit_path', required=True)
    parser.add_argument('--run-id', required=True)
    args = parser.parse_args()
    try:
        validate(read(args.summary), read(args.exit_path), args.run_id)
    except (OSError, ValidationError):
        raise SystemExit('Permission strict B evidence failed; native-only or uncovered is not PASS') from None
    print('Permission schema14 strict B artifact and matching exit accepted; native provenance remains source/run-bound.')


if __name__ == '__main__':
    main()
