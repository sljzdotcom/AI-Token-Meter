"""UNRUN real-watchdog regression. One case; no mock classification or PID signals."""
import errno, fcntl, hashlib, json, os, re, signal, stat, subprocess, sys, time
from pathlib import Path

OWNED = None
ROOT = None
EVENTS = []
SIGNALS = []
GATE_FD = None
LIMIT = 32768
EXPECTED_PROGRAM_SHA256 = {'7a743a27fcc5848895b1ddcb48fd01c038138212b961c05e410638bd361bc5bc', 'c59199753b6f45107d115a830f2d3d3e6ad1bd697012e947523288ef900170f6'}
EXPECTED_CASE_TABLE_SHA256 = '0a6c1ec29edd49c073df0e1c7bd4fa5c72d5c1b4f1a5b13026ba34a4b9c33f7d'
CASE_SECONDS = 6
DEADLINE_SECONDS = 1

def require(value):
    if not value:
        raise ValueError('watchdog_fixture_precondition')

def sha(data):
    return hashlib.sha256(data).hexdigest()

def read_private(path, capacity):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as f:
        before = os.fstat(f.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_uid == os.getuid()
                and before.st_nlink == 1 and stat.S_IMODE(before.st_mode) == 0o600
                and 0 <= before.st_size <= capacity)
        data = f.read(capacity + 1)
        after = os.fstat(f.fileno())
        named = path.lstat()
    identity = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_nlink,
                          s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(identity(before) == identity(after) == identity(named) and len(data) == before.st_size)
    return data

def create(path, data=b''):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), 'wb') as f:
        f.write(data); f.flush(); os.fsync(f.fileno())

def save_record(name, record):
    create(ROOT / name, (json.dumps(record, sort_keys=True, indent=2) + '\n').encode())

def extract_watchdog(data):
    # Shared by source-only RED/GREEN and the native test's actual emitted script.
    marker = b" -e 'use Fcntl qw(:DEFAULT); use Time::HiRes qw(time); "
    tail = b"' \"$$\" "
    require(data.count(marker) == 1)
    start = data.index(marker) + len(b" -e '")
    end = data.index(tail, start)
    program = data[start:end]
    require(0 < len(program) <= 8192 and sha(program) in EXPECTED_PROGRAM_SHA256
            and b"'" not in program and b'\\' not in program)
    text = program.decode('ascii')
    # No program rewriting/interception. Every signal target is the real parent.
    kills = re.findall(r'\bkill\b', text)
    targeted = re.findall(r'\bkill\s+"USR[12]",\s*\$parent_pid\b', text)
    require(len(kills) > 0 and len(kills) == len(targeted)
            and 'getppid() != $parent_pid' in text
            and not re.search(r'\b(?:fork|exec|system|qx)\b|`', text))
    return program, [start, end]

def actual_signal(signum, unused_frame):
    # Python invokes this on its own main thread; it does not alter watchdog signals.
    SIGNALS.append({'name': 'USR1' if signum == signal.SIGUSR1 else 'USR2',
                    'monotonic': time.monotonic()})

def retain(reason):
    # This controller remains alive, so its PID cannot be recycled under watchdog.
    try:
        save_record('incomplete.json', {'reason': reason, 'controller_pid': os.getpid(),
                    'watchdog_pid': OWNED.pid if OWNED else None,
                    'observed_returncode': OWNED.returncode if OWNED else None,
                    'events': EVENTS, 'signals': SIGNALS, 'lifecycle_complete': False,
                    'no_retry_or_next': True, 'signals_sent_by_harness': []})
    except BaseException:
        pass
    while True:
        try:
            signal.pause()
        except BaseException:
            pass

def check_live(deadline):
    if time.monotonic() >= deadline:
        retain('bounded_case_deadline')
    if OWNED.returncode is None:
        OWNED.poll()  # Direct child only; Popen caches an already known reap.
    if OWNED.returncode is not None:
        retain('watchdog_ended_before_planned_read')

def fifo_writer(path, deadline):
    while True:
        check_live(deadline)
        try:
            fd = os.open(path, os.O_WRONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
        except OSError as error:
            if error.errno not in (errno.ENXIO, errno.EINTR):
                raise
            time.sleep(0.002)
            continue
        s = os.fstat(fd)
        require(stat.S_ISFIFO(s.st_mode) and s.st_uid == os.getuid()
                and stat.S_IMODE(s.st_mode) == 0o600 and s.st_nlink == 1)
        return fd

def send_bytes(fd, data, deadline):
    offset = 0
    while offset < len(data):
        check_live(deadline)
        try:
            count = os.write(fd, data[offset:])
        except (BlockingIOError, InterruptedError):
            continue
        require(count > 0)
        offset += count

def frame(number, kind, deadline, cross_deadline=False):
    # Opening a writer proves watchdog is at this exact read_status request read.
    fd = fifo_writer(ROOT / 'request.fifo', deadline)
    opened_wall = time.time()
    if cross_deadline:
        # Read2 follows creation of the real runtime +1s deadline. Keep its read
        # blocked until that real wall deadline has passed; no clock substitution.
        release_wall = opened_wall + DEADLINE_SECONDS + 0.15
        while time.time() < release_wall:
            check_live(deadline)
            time.sleep(0.002)
    request = b'cancelled\n' if kind == 'cancel' else b'cancelled_extra\n'
    send_bytes(fd, request, deadline)
    os.close(fd)
    if kind != 'cancel':
        state = {'diagnostic': b'process_state_unavailable\n',
                 'stop-unconfirmed': b'helper_stop_unconfirmed\n',
                 'empty': b'', 'unknown': b'unrecognized_state\n', 'ready': b'ready\n'}[kind]
        status_fd = fifo_writer(ROOT / 'status.fifo', deadline)
        send_bytes(status_fd, state, deadline)
        os.close(status_fd)
    EVENTS.append({'read_number': number, 'input': kind, 'request': request.decode().strip(),
                   'opened_wall': opened_wall, 'released_wall': time.time(),
                   'crossed_real_deadline': cross_deadline})

def main():
    global ROOT, OWNED, GATE_FD
    require(sys.platform == 'darwin' and os.getuid() == os.geteuid() > 0 and len(sys.argv) == 3)
    requested_root = Path(sys.argv[1])
    require(requested_root.is_absolute())
    # Foundation may spell a private temporary URL with the /var alias.
    ROOT = requested_root.resolve(strict=True)
    require(Path(__file__).resolve(strict=True) == ROOT / 'watchdog_state_regression.py')
    s = ROOT.lstat()
    require(stat.S_ISDIR(s.st_mode) and s.st_uid == os.getuid() and stat.S_IMODE(s.st_mode) == 0o700)
    os.umask(0o077)
    source = read_private(ROOT / 'input.txt', 1048576)
    table_raw = read_private(ROOT / 'watchdog_state_cases.json', 16384)
    require(sha(table_raw) == EXPECTED_CASE_TABLE_SHA256)
    cases = json.loads(table_raw)
    rows = [row for row in cases if row['id'] == sys.argv[2]]
    require(len(rows) == 1 and len(cases) == 28)
    case = rows[0]
    require(case['phase'] in ('startup', 'running', 'deadline-unlocked', 'deadline-contended', 'deadline-missing-gate')
            and case['input'] in ('diagnostic', 'stop-unconfirmed', 'cancel', 'empty', 'unknown', 'ready')
            and case['expected_signals'] in ([], ['USR1'], ['USR2']))
    program, offsets = extract_watchdog(source)
    create(ROOT / 'extracted-watchdog.pl', program)
    for name in ('request.fifo', 'status.fifo'):
        os.mkfifo(ROOT / name, 0o600)
    create(ROOT / 'group.marker', b'synthetic-existence-marker-only\n')
    GATE_FD = os.open(ROOT / 'gate.lock', os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    contended = case['phase'] == 'deadline-contended'
    if contended:
        fcntl.lockf(GATE_FD, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if case['phase'] == 'deadline-missing-gate':
        os.unlink(ROOT / 'gate.lock')  # This fixture's freshly created private file only.
    signal.signal(signal.SIGUSR1, actual_signal)
    signal.signal(signal.SIGUSR2, actual_signal)
    signal.pthread_sigmask(signal.SIG_UNBLOCK, {signal.SIGUSR1, signal.SIGUSR2})
    argv = ['/usr/bin/perl', '-e', program.decode('ascii'), str(os.getpid()), str(DEADLINE_SECONDS),
            str(ROOT / 'status.fifo'), str(ROOT / 'request.fifo'), str(ROOT / 'ready.marker'),
            str(ROOT / 'group.marker'), str(ROOT / 'gate.lock')]
    save_record('started.json', {'case': case, 'controller_pid': os.getpid(), 'argv': argv,
                'source_sha256': sha(source), 'program_sha256': sha(program), 'program_bytes': len(program),
                'source_byte_range': offsets, 'case_table_sha256': sha(table_raw),
                'gate_lock_held_by_controller': contended, 'timeout_seconds': CASE_SECONDS,
                'harness_sends_no_signals': True})
    out = os.open(ROOT / 'watchdog.stdout', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    err = os.open(ROOT / 'watchdog.stderr', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    env = {'PATH': '/usr/bin:/bin:/usr/sbin:/sbin', 'HOME': str(ROOT), 'TMPDIR': str(ROOT) + '/', 'LANG': 'C', 'LC_ALL': 'C'}
    OWNED = subprocess.Popen(argv, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                             stdout=out, stderr=err, close_fds=True)
    os.close(out); os.close(err)
    save_record('child.json', {'pid': OWNED.pid, 'direct_owned_child': True,
                'parent_pid_argv_is_this_live_controller': os.getpid()})
    deadline = time.monotonic() + CASE_SECONDS
    phase, kind = case['phase'], case['input']
    if phase == 'startup':
        frame(1, kind, deadline)
    else:
        frame(1, 'ready', deadline)
        if phase == 'running':
            frame(2, kind, deadline)
        else:
            frame(2, 'ready', deadline, cross_deadline=True)
            frame(3, kind, deadline)
            if contended and kind == 'ready':
                for number in range(4, 23):
                    frame(number, 'ready', deadline)
    while OWNED.returncode is None:
        if time.monotonic() >= deadline:
            retain('watchdog_not_reaped_within_bound')
        OWNED.poll()
        if OWNED.returncode is None:
            time.sleep(0.002)
    # All watchdog signal sends precede its now-observed real exit/reap.
    if signal.sigpending() & {signal.SIGUSR1, signal.SIGUSR2}:
        retain('signal_delivery_pending_after_reap')
    stdout = read_private(ROOT / 'watchdog.stdout', LIMIT)
    stderr = read_private(ROOT / 'watchdog.stderr', LIMIT)
    names = [row['name'] for row in SIGNALS]
    passed = OWNED.returncode == 0 and not stderr and names == case['expected_signals']
    record = {'case': case, 'controller_pid': os.getpid(), 'watchdog_pid': OWNED.pid,
              'watchdog_returncode': OWNED.returncode, 'watchdog_reaped': True,
              'lifecycle_complete': True, 'signals': SIGNALS, 'signal_names': names,
              'semantic_passed': passed, 'events': EVENTS, 'program_sha256': sha(program),
              'logs': {name: {'bytes': len(data), 'sha256': sha(data)}
                       for name, data in [('stdout', stdout), ('stderr', stderr)]},
              'signals_sent_by_harness': [], 'automatic_next': False}
    save_record('receipt.json', record)
    os.close(GATE_FD); GATE_FD = None
    print(json.dumps({'case': case['id'], 'semantic_passed': passed, 'signal_names': names,
                      'watchdog_reaped': True}, sort_keys=True), flush=True)
    return 0 if passed else 1

if __name__ == '__main__':
    try:
        result = main()
    except BaseException as error:
        if OWNED is not None:
            retain(type(error).__name__)
        result = 2
    sys.exit(result)
