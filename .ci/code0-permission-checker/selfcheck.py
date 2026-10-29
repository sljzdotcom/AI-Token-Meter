"""UNRUN schema14 DATA behavior definitions, written before validator source.

These canaries are synthetic parser DATA, never native permission evidence.
The fixtures are literal contract expectations; validator does not build them.
"""
import copy
import io
import json
import unittest

import validator as subject


def positive(branch='ZeroObserved'):
    data = {
        'schema': 14, 'runId': 'canary',
        'requiredCase': 'refusal-notification-owned-https',
        'selectedCases': 1, 'completedCases': 1, 'pass': True, 'failure': 'none',
        'install': {
            'submissions': 0, 'barrierReceipts': 0, 'firstLoadReceipts': 0,
            'documentReceipts': 0, 'submissionAccepted': False,
            'installRejected': False, 'scriptAcknowledged': False,
            'cdpAcknowledged': False, 'sourceMatched': False,
            'syntheticResponseApplied': False, 'canonicalReadyControl': False,
            'ackBeforeFirstLoad': False,
        },
        'lifecycle': {
            'staMain': True, 'callbackAffinity': True, 'noInheritedOverride': True,
            'engineReady': True, 'bareHiddenUnfocused': True,
            'constructionObserved': True, 'profileIsolated': True,
            'destroyed': True, 'ownedReferencesReleased': True,
            'runReturned': True, 'cleanupSucceeded': True,
            'bareWindowViolation': False, 'runtimeVersion': '155.0.1.2',
        },
        'nativeRefusal': {
            'receipts': 1, 'nativeInstanceValidated': True,
            'sourceBoundRegistry': True, 'navigationNotIssued': False,
            'loadRejected': False, 'requiredMask': 1023, 'installedMask': 1023,
            'notInstalledHooks': [],
        },
        'permission': {
            'version': 1, 'selector': 'refusal-notification-owned-https',
            'phase': 'Closed', 'unknown': False, 'overflow': False,
            'setup': {
                'hooks1023': 1023, 'settingsAck': True, 'responderReady': True,
                'receiversReady': True, 'domainsAck': True, 'documentBound': True,
                'lifecycleAssemblyComplete': True, 'blankBranchClosed': True,
                'observationCoverageIntactAtArm': True, 'originalDeadlineBound': True,
            },
            'counts': {
                'navigation': 1, 'mainResponses': 1, 'permissionEntries': 1,
                'kindReads': 1, 'gestureReads': 1, 'originReads': 1,
                'stateBeforeReads': 1, 'denyAttempts': 1, 'denyReturns': 1,
                'denySuccesses': 1, 'stateAfterReads': 1, 'actionReceipts': 1,
                'evaluateSubmissions': 1, 'evaluateCompletions': 1,
                'effectReceipts': 1, 'sessionPolicyFailures': 1,
                'finalReceipts': 1, 'unexpectedRequests': 0,
                'receiverAdds': 10, 'receiverRemoves': 10, 'commandIssued': 4,
                'commandReturned': 4, 'commandCompleted': 4, 'commandConsumed': 4,
            },
            'native': dict.fromkeys((
                'senderMatched', 'requestInstallationMatched', 'kindNotifications',
                'nonUser', 'originMatched', 'stateBeforeDefault', 'denySucceeded',
                'stateAfterDeny', 'originalTimely', 'postDropMatched',
                'enteredBeforeObservationCutoff', 'capturedBeforeObservationCutoff',
                'qualifiedBeforeObservationCutoff'), True),
            'effect': dict.fromkeys((
                'successHresult', 'closedDeniedResult', 'sameTicket',
                'sameFrameLoaderContext', 'sourceMatched', 'invokedAfterNativeAction',
                'invokedBeforeStop', 'capturedBeforeStop', 'qualifiedBeforeStop',
                'enteredBeforeObservationCutoff', 'capturedBeforeObservationCutoff',
                'qualifiedBeforeObservationCutoff', 'originalTimely', 'closeTimely',
                'epochUnchangedThroughQualification'), True),
            'ordering': dict.fromkeys((
                'permanentRevocationAtRefusal', 'firstCloseAtRefusal',
                'noActionReauthorized', 'noWaitForEffect', 'actualPolicyFailureAfterAction',
                'cutoffLatched', 'cutoffBeforeAnyRequiredRemoval',
                'qualifiedObservationsAreHistorical', 'physicalCloseObserved'), True),
            'drain': dict.fromkeys((
                'obligationsEmpty', 'handlersEmpty', 'textEmpty',
                'receiverTokensRemoved', 'resourceAndFilterRemoved', 'controlsEmpty'), True),
            'coverage': {
                'requiredMask': 16777215, 'addedMask': 16777215,
                'removeAttemptMask': 16777215, 'removedMask': 16777215,
                'uncertainMask': 0, 'ownedMask': 0,
                'addAttempts': [1] * 24, 'addReturns': [True] * 24,
                'addAcks': [1] * 24, 'removeAttempts': [1] * 24,
                'removeReturns': [True] * 24, 'removeAcks': [1] * 24,
            },
            'commands': {
                'issued': [1] * 4, 'returned': [1] * 4,
                'submissionSucceeded': [True] * 4, 'completed': [1] * 4,
                'completionSucceeded': [True] * 4, 'consumed': [1] * 4,
                'handlerDrops': [1] * 4,
                'values': ['emptyAck', 'emptyAck', 'frameTree', 'denied'],
                'qualified': [True] * 4, 'obligations': 0, 'handlers': 0,
                'inFlight': 0,
            },
            'observations': {
                'blankBranch': branch, 'bootstrapDeliveries': 4,
                'disposalDeliveries': 0, 'deliveries': 4,
                'byReceiver': [1, 0, 0, 0, 0, 1, 0, 0, 1, 1],
                'postArmPreCutoff': 0, 'counterOverflow': False,
            },
            'inventory': {
                'textBytes': 0, 'textPeakBytes': 2000, 'textReserveFailures': 0,
                'textAccountingComplete': True, 'activeGuards': 0,
                'aggregateHandlers': 0, 'newEventHandlers': 0, 'commandHandlers': 0,
                'handlerAccountingComplete': True, 'recordSlots': 2,
                'recordSlotsPeak': 3, 'completionSlot': 0,
                'documentRecord': 1, 'nativeRecord': 1,
            },
            'timing': {
                'nativeEnteredAtMs': 100, 'nativeCapturedAtMs': 102,
                'nativeQualifiedAtMs': 103, 'effectEnteredAtMs': 104,
                'effectCapturedAtMs': 105, 'effectQualifiedAtMs': 106,
                'firstCloseAtMs': 101, 'observationCutoffAtMs': 107,
                'stopEnteredAtMs': 108, 'controllerCloseEnteredAtMs': 109,
                'releasedAtMs': 110, 'finalAtMs': 112, 'cleanupElapsedMs': 11,
                'finalWithinCloseDeadline': True, 'sameOriginalClock': True,
                'firstCloseAdopted': True,
            },
            'status': {
                'protocolPhase': 'Released', 'protocolFailure': None,
                'observationEpoch': 1, 'cutoffReason': 'RequiredRemovalAttempt',
                'nativeQualified': True, 'effectQualified': True,
                'effectResult': 'denied', 'sessionReason': 'PolicyRejected',
                'navigationSucceeded': True, 'mainResponseSucceeded': True,
                'terminalResult': 'succeeded',
            },
            'cleanup': {
                'browserProcessExits': 1, 'windowDestroyed': 1,
                'releasedRefZero': True, 'wakeShutdownIssued': True,
                'wakeJoins': 1, 'wakeJoinSucceeded': True,
                'tempDirCloseAttempts': 1, 'tempDirCloseSucceeded': True,
                'tempDirResultDropped': True,
            },
        },
    }
    if branch == 'Observed':
        data['permission']['observations'].update(
            bootstrapDeliveries=6, deliveries=6,
            byReceiver=[2, 1, 0, 0, 0, 1, 0, 0, 1, 1])
    return data


def exit_canary():
    return {'schema': 1, 'runId': 'canary', 'exitCode': 0}


def encode(data):
    return json.dumps(data, separators=(',', ':'), ensure_ascii=True).encode('ascii')


def nodes(value, path=()):
    yield path, value
    if type(value) is dict:
        for key, child in value.items():
            yield from nodes(child, path + (key,))
    elif type(value) is list:
        for index, child in enumerate(value):
            yield from nodes(child, path + (index,))


def at(data, path):
    for key in path:
        data = data[key]
    return data


def changed(data, path, value):
    result = copy.deepcopy(data)
    if not path:
        return value
    at(result, path[:-1])[path[-1]] = value
    return result


def duplicate_json(value, target, path=()):
    if type(value) is dict:
        rows = [(json.dumps(k), duplicate_json(v, target, path + (k,)))
                for k, v in value.items()]
        if path == target:
            rows.append(rows[0])
        return '{' + ','.join(k + ':' + v for k, v in rows) + '}'
    if type(value) is list:
        return '[' + ','.join(duplicate_json(v, target, path + (i,))
                              for i, v in enumerate(value)) + ']'
    return json.dumps(value, separators=(',', ':'))


class PermissionDataChecks(unittest.TestCase):
    """Every negative keeps pass=true unless explicitly testing pass itself."""

    def accept(self, data, exit_data=None):
        subject.validate(subject.decode(encode(data)),
                         subject.decode(encode(exit_data or exit_canary())), 'canary')

    def reject(self, data, exit_data=None):
        with self.assertRaises(subject.ValidationError):
            subject.validate(data, exit_data if exit_data is not None else exit_canary(),
                             'canary')

    # Mutation: make blank-branch selection a preferred branch or one fixture value.
    def test_both_branches_and_nonconstant_legal_observations(self):
        for branch in ('ZeroObserved', 'Observed'):
            data = positive(branch)
            self.accept(data)
            obs = data['permission']['observations']
            if branch == 'Observed':
                obs.update(bootstrapDeliveries=8, disposalDeliveries=2, deliveries=10,
                           byReceiver=[2, 1, 0, 0, 0, 3, 0, 0, 2, 2])
            else:
                obs.update(bootstrapDeliveries=5, disposalDeliveries=2, deliveries=7,
                           byReceiver=[1, 0, 1, 0, 0, 3, 0, 0, 1, 1])
            self.accept(data)

    # Mutation: hard-code the canary peak/history or incorrectly require history erasure.
    def test_peak_history_and_normal_cutoff_ranges(self):
        for peak in (1, 4096):
            for retained in (0, 1, 2):
                for reason in ('RequiredRemovalAttempt', 'ControlsDisposal',
                               'StopEntry', 'ControllerCloseEntry'):
                    data = positive()
                    inv = data['permission']['inventory']
                    inv.update(textPeakBytes=peak, recordSlots=retained,
                               recordSlotsPeak=2, documentRecord=int(retained > 0),
                               nativeRecord=int(retained > 1))
                    data['permission']['status']['cutoffReason'] = reason
                    self.accept(data)

    # Mutation: rounded subtraction becomes an authority or same-ms ordering is rejected.
    def test_precise_flags_allow_rounded_equalities_and_final_after45(self):
        data = positive()
        t = data['permission']['timing']
        for key in t:
            if key.endswith('AtMs'):
                t[key] = 44999
        t.update(finalAtMs=45001, cleanupElapsedMs=1)
        self.accept(data)
        t['finalWithinCloseDeadline'] = False
        self.reject(data)

    # Mutation: omit any one actual mandatory flag/count or relax its exact type.
    def test_every_required_leaf_unavailable_or_wrong_type_is_rejected(self):
        base = positive()
        for path, value in nodes(base):
            if type(value) in (dict, list):
                continue
            with self.subTest(path=path, mutation='null'):
                if value is not None:
                    self.reject(changed(base, path, None))
            if type(value) is bool:
                self.reject(changed(base, path, not value))
                self.reject(changed(base, path, int(value)))
            if type(value) is str or value is None:
                self.reject(changed(base, path, 'unknown-enum!'))
            if type(value) is int:
                if path[:2] != ('permission', 'timing') and path != (
                        'permission', 'inventory', 'textPeakBytes'):
                    self.reject(changed(base, path, value + 1))
                for bad in (True, False, -1, 1.0, 18446744073709551616):
                    self.reject(changed(base, path, bad))

    # Mutation: object/array acceptance becomes open, recursive key checks disappear.
    def test_every_object_keyset_duplicate_and_every_array_length(self):
        base = positive()
        for path, value in nodes(base):
            if type(value) is dict:
                for wrong in (None, False, [], 'object'):
                    self.reject(changed(base, path, wrong))
                for key in value:
                    bad = copy.deepcopy(base)
                    del at(bad, path)[key]
                    self.reject(bad)
                bad = copy.deepcopy(base)
                at(bad, path)['unexpected'] = True
                self.reject(bad)
                with self.assertRaises(subject.ValidationError):
                    subject.decode(duplicate_json(base, path).encode('ascii'))
            elif type(value) is list:
                for wrong in (None, False, {}, 'array'):
                    self.reject(changed(base, path, wrong))
                self.reject(changed(base, path, value + [None]))
                if value:
                    self.reject(changed(base, path, value[:-1]))

    # Mutation: one of24 true native coverage obligations is substituted by a mask.
    def test_each_coverage_row_and_mask_is_mandatory(self):
        base = positive()
        for row in range(24):
            for key in ('addAttempts', 'addAcks', 'removeAttempts', 'removeAcks'):
                self.reject(changed(base, ('permission', 'coverage', key, row), 0))
            for key in ('addReturns', 'removeReturns'):
                self.reject(changed(base, ('permission', 'coverage', key, row), False))
            for key in ('requiredMask', 'addedMask', 'removeAttemptMask', 'removedMask'):
                self.reject(changed(base, ('permission', 'coverage', key),
                                    16777215 ^ (1 << row)))
            for key in ('ownedMask', 'uncertainMask'):
                self.reject(changed(base, ('permission', 'coverage', key), 1 << row))

    # Mutation: parsing denied alone overrides missing native/Session or failed HRESULT.
    def test_strict_B_requires_every_native_effect_session_and_final_receipt(self):
        base = positive()
        for key in base['permission']['counts']:
            value = base['permission']['counts'][key]
            self.reject(changed(base, ('permission', 'counts', key), value + 1))
        for key in ('permissionEntries', 'denyAttempts', 'denySuccesses', 'stateAfterReads',
                    'actionReceipts', 'effectReceipts', 'sessionPolicyFailures', 'finalReceipts'):
            self.reject(changed(base, ('permission', 'counts', key), 0))
        for result in ('precondition', 'default', 'granted', 'documentChanged', 'invalid',
                       'malformed', None):
            bad = changed(base, ('permission', 'commands', 'values', 3), result)
            bad['permission']['status']['effectResult'] = result
            self.reject(bad)
        for key, value in (('nativeQualified', False), ('effectQualified', False),
                           ('sessionReason', 'other'), ('protocolFailure', 'Result'),
                           ('cutoffReason', 'CoverageLost'), ('cutoffReason', 'AffinityLost')):
            self.reject(changed(base, ('permission', 'status', key), value))

    # Mutation: callbacks/handlerDrop may erase unconsumed work or command identity.
    def test_all_four_command_rows_and_obligations_are_independent(self):
        base = positive()
        for row in range(4):
            for key in ('issued', 'returned', 'completed', 'consumed', 'handlerDrops'):
                self.reject(changed(base, ('permission', 'commands', key, row), 0))
            for key in ('submissionSucceeded', 'completionSucceeded', 'qualified'):
                self.reject(changed(base, ('permission', 'commands', key, row), False))
            self.reject(changed(base, ('permission', 'commands', 'values', row), 'wrong'))
        for key in ('obligations', 'handlers', 'inFlight'):
            self.reject(changed(base, ('permission', 'commands', key), 1))

    # Mutation: timestamp ordering/45/10/adoption gates or post-cleanup timing disappear.
    def test_deadline_order_and_rounding_negatives(self):
        base = positive()
        for key, value in (
                ('nativeEnteredAtMs', 45000), ('nativeCapturedAtMs', 45000),
                ('nativeQualifiedAtMs', 45000), ('effectEnteredAtMs', 45000),
                ('effectCapturedAtMs', 45000), ('effectQualifiedAtMs', 45000),
                ('nativeCapturedAtMs', 99), ('nativeQualifiedAtMs', 105),
                ('firstCloseAtMs', 99), ('firstCloseAtMs', 103),
                ('observationCutoffAtMs', 105), ('stopEnteredAtMs', 106),
                ('controllerCloseEnteredAtMs', 107), ('releasedAtMs', 108),
                ('finalAtMs', 109), ('cleanupElapsedMs', 10000),
                ('cleanupElapsedMs', 9), ('cleanupElapsedMs', 12)):
            self.reject(changed(base, ('permission', 'timing', key), value))
        for key in ('finalWithinCloseDeadline', 'sameOriginalClock', 'firstCloseAdopted'):
            self.reject(changed(base, ('permission', 'timing', key), False))
        # Native release can be timely while actual final cleanup expires.
        late = copy.deepcopy(base)
        late['permission']['timing'].update(finalAtMs=10101, cleanupElapsedMs=10000)
        self.reject(late)

    # Mutation: unknown accounting/leak/saturation is silently normalized to zero.
    def test_inventory_bounds_overflow_and_coupled_records(self):
        base = positive()
        for key, value in (('textBytes', 1), ('textPeakBytes', 0), ('textPeakBytes', 4097),
                           ('textReserveFailures', 1), ('activeGuards', 1),
                           ('aggregateHandlers', 1), ('newEventHandlers', 1),
                           ('commandHandlers', 1), ('completionSlot', 1),
                           ('recordSlots', 1), ('recordSlotsPeak', 1),
                           ('documentRecord', 2), ('nativeRecord', 2)):
            self.reject(changed(base, ('permission', 'inventory', key), value))
        for key in ('unknown', 'overflow'):
            self.reject(changed(base, ('permission', key), True))
        for key, value in (('bootstrapDeliveries', 17), ('postArmPreCutoff', 1),
                           ('deliveries', 5), ('counterOverflow', True)):
            self.reject(changed(base, ('permission', 'observations', key), value))

    # Mutation: false cleanup/physical boundary facts are optional diagnostics.
    def test_real_cleanup_and_P1_shared_semantics_cannot_be_substituted(self):
        base = positive()
        for key, value in base['permission']['cleanup'].items():
            self.reject(changed(base, ('permission', 'cleanup', key),
                                False if type(value) is bool else 0))
        for key, value in base['install'].items():
            self.reject(changed(base, ('install', key),
                                True if type(value) is bool else 1))
        for group in ('setup', 'native', 'effect', 'ordering', 'drain'):
            for key, value in base['permission'][group].items():
                if type(value) is bool:
                    self.reject(changed(base, ('permission', group, key), False))

    # Mutation: bound run/selector/runtime or actual process exit is inferred from summary.
    def test_fresh_run_exit_runtime_and_no_other_schema_fallback(self):
        base = positive()
        for schema in (0, 1, 13, 15, 16):
            self.reject(changed(base, ('schema',), schema))
        for path in (('runId',), ('requiredCase',), ('permission', 'selector')):
            self.reject(changed(base, path, 'stale'))
        for runtime in ('', 'x', '1' * 129):
            self.reject(changed(base, ('lifecycle', 'runtimeVersion'), runtime))
        for expected in ('', 'with space', 'x' * 65, None, True):
            with self.assertRaises(subject.ValidationError):
                subject.validate(base, exit_canary(), expected)
        for missing_or_wrong in (None, False, 0, [], 'exit'):
            with self.assertRaises(subject.ValidationError):
                subject.validate(base, missing_or_wrong, 'canary')
        for path, value in nodes(exit_canary()):
            if path:
                self.reject(base, changed(exit_canary(), path, 'stale'))
                self.reject(base, changed(exit_canary(), path, True))
        self.reject(base, {**exit_canary(), 'exitCode': 1})
        self.reject(base, {**exit_canary(), 'extra': True})
        for key in exit_canary():
            self.reject(base, {k: v for k, v in exit_canary().items() if k != key})

    # Mutation: decode bypasses cap/ASCII/unique decoded members/number/depth grammar.
    def test_raw_JSON_duplicate_depth_number_and_encoding_negatives(self):
        raw_bad = (b'\xef\xbb\xbf{}', b'\xff', b'{}{}', b'{} trailing',
                   b'{"x":1,"\\u0078":1}', b'{"x":NaN}', b'{"x":Infinity}',
                   b'{"x":-Infinity}', b'{"x":1.0}', b'{"x":1e99}',
                   b' ' * 8193, b'[[[[[]]]]]', b'{' * 1000)
        for raw in raw_bad:
            with self.assertRaises(subject.ValidationError):
                subject.decode(raw)
        with self.assertRaises(subject.ValidationError):
            subject.decode(duplicate_json(exit_canary(), ()).encode('ascii'))
        raw = encode(positive())
        padded = b' ' * (8192 - len(raw)) + raw
        subject.validate(subject.decode(padded), exit_canary(), 'canary')
        subject.validate(subject.decode(raw + b'\n'), exit_canary(), 'canary')
        escaped_known = raw.replace(b'"schema":14', b'"\\u0073chema":14', 1)
        subject.validate(subject.decode(escaped_known), exit_canary(), 'canary')
        with self.assertRaises(subject.ValidationError):
            subject.decode(padded + b' ')

    # Mutation: reader consumes an unbounded file before deciding its size.
    def test_reader_requests_only8193_and_rejects_the_extra_byte(self):
        class ReadWitness(io.BytesIO):
            requested = None

            def read(self, count=-1):
                self.requested = count
                return super().read(count)

        raw = encode(positive())
        stream = ReadWitness(raw)
        subject.validate(subject.bounded_read(stream), exit_canary(), 'canary')
        self.assertEqual(stream.requested, 8193)
        stream = ReadWitness(b' ' * 8193)
        with self.assertRaises(subject.ValidationError):
            subject.bounded_read(stream)
        self.assertEqual(stream.requested, 8193)

    # Mutation: arbitrary per-receiver16/2 cap, S16 overflow or runtime/run string bounds.
    def test_disposal_S16_endpoints_and_run_runtime_limits(self):
        data = positive()
        data['permission']['observations'].update(
            disposalDeliveries=65531, deliveries=65535,
            byReceiver=[65532, 0, 0, 0, 0, 1, 0, 0, 1, 1])
        self.accept(data)
        data['permission']['observations'].update(
            disposalDeliveries=65532, deliveries=65536,
            byReceiver=[65533, 0, 0, 0, 0, 1, 0, 0, 1, 1])
        self.reject(data)
        for run in ('r', 'r' * 64):
            for runtime in ('1', '1' * 128):
                data = positive()
                data['runId'] = run
                data['lifecycle']['runtimeVersion'] = runtime
                subject.validate(data, {'schema': 1, 'runId': run, 'exitCode': 0}, run)

    # Mutation: partially failed facts are guessed positive, or failure shapes become open.
    def test_bounded_failure_shape_is_inspectable_but_never_accepted(self):
        data = positive()
        data['pass'] = False
        data['failure'] = 'permissionEffectLate'
        data['permission']['status'].update(effectQualified=False, terminalResult='failed')
        data['permission']['effect']['qualifiedBeforeObservationCutoff'] = False
        data['permission']['commands']['qualified'][3] = False
        subject.check_shape(data, exit_canary(), 'canary')
        self.reject(data)
        data['permission']['effect']['invented'] = True
        with self.assertRaises(subject.ValidationError):
            subject.check_shape(data, exit_canary(), 'canary')

    # Mutation: aggregate equality incorrectly sums saturated C lower bounds.
    def test_saturated_partial_counts_preserve_truthful_lower_bounds(self):
        data = positive()
        data['pass'] = False
        data['failure'] = 'permissionOverflow'
        p = data['permission']
        p['overflow'] = True
        p['status']['terminalResult'] = 'failed'
        for key in ('issued', 'returned', 'completed', 'consumed'):
            p['commands'][key] = [2, 0, 0, 0]
        for key in ('commandIssued', 'commandReturned', 'commandCompleted', 'commandConsumed'):
            p['counts'][key] = 5
        p['counts']['evaluateSubmissions'] = 0
        p['counts']['evaluateCompletions'] = 0
        subject.check_shape(data, exit_canary(), 'canary')
        self.reject(data)
        p['counts']['commandIssued'] = 1
        with self.assertRaises(subject.ValidationError):
            subject.check_shape(data, exit_canary(), 'canary')


    # Independent literal oracle: every contract exact-once fact needs a legal
    # zero history, not merely a type error or C=2 saturation rejection.
    def test_every_exact_once_count_zero_and_two_without_losing_pass(self):
        required_once = (
            'navigation', 'mainResponses', 'permissionEntries', 'kindReads',
            'gestureReads', 'originReads', 'stateBeforeReads', 'denyAttempts',
            'denyReturns', 'denySuccesses', 'stateAfterReads', 'actionReceipts',
            'evaluateSubmissions', 'evaluateCompletions', 'effectReceipts',
            'sessionPolicyFailures', 'finalReceipts',
        )
        for key in required_once:
            with self.subTest(counter=key, value=0):
                data = positive()
                p = data['permission']
                p['counts'][key] = 0
                # Missing attempt/return requires a coherent shorter native
                # history. These cases exercise the conjunction of DENY facts.
                if key == 'denyAttempts':
                    p['counts'].update(denyReturns=0, denySuccesses=0)
                elif key == 'denyReturns':
                    p['counts']['denySuccesses'] = 0
                # The evaluation counters have explicit command-row joins.
                # Keep the whole absent operation coherent, then require the
                # positive gate to reject it despite retained pass=true.
                elif key in ('evaluateSubmissions', 'evaluateCompletions'):
                    p['counts'].update(evaluateSubmissions=0, evaluateCompletions=0,
                                       commandIssued=3, commandReturned=3,
                                       commandCompleted=3, commandConsumed=3)
                    for row in ('issued', 'returned', 'completed', 'consumed', 'handlerDrops'):
                        p['commands'][row][3] = 0
                    p['commands']['values'][3] = None
                    p['status']['effectResult'] = None
                self.assertIs(data['pass'], True)
                subject.check_shape(data, exit_canary(), 'canary')
                self.reject(data)
            with self.subTest(counter=key, value=2):
                data = changed(positive(), ('permission', 'counts', key), 2)
                self.assertIs(data['pass'], True)
                self.reject(data)


if __name__ == '__main__':
    unittest.main()
