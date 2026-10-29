#!/usr/bin/env python3
"""Opt-in, Windows GitHub CI only; four fixed dispatcher semantic mutations.

Never run this on a developer host. See the preparation report for authorization,
limits, and invocation. No native/STA/release acceptance is implied.
"""
from __future__ import annotations

import difflib
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time

DISPATCHER = "windows/src-tauri/src/platform/windows/code0_dispatcher.rs"
TESTS = "windows/src-tauri/tests/code0_dispatcher.rs"
DISPATCHER_SHA = "1ce7229523ee0f010ecc198b2d9e8d24188be8dc9ac3c0f9dd7539bd80ce4355"
TESTS_SHA = "b736e324815a8e838881e0592c79c74f422bb872534b44248813088863921305"
# Already-tracked historical reports remain byte-compared source inputs. Only
# these exact files and their directory entries may use the reserved prefix.
HISTORICAL_REPORT_FILES = frozenset((
    ".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/task-3-report.md",
    ".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/task-4-report.md",
))
HISTORICAL_REPORT_DIRS = frozenset((
    ".superpowers", ".superpowers/sdd",
    ".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density",
))
PRIVATE = "platform::windows::code0_dispatcher::tests::"
CHANGED = "disconnect_reply_observes_new_null_revision_after_matching_emit"
COMMIT = b"                *self.proxy.view.lock().unwrap() = view.clone();\n"
EMIT = b"                (self.emit.borrow_mut())(&view);\n"
REPLY = b"        if let Some((reply, value)) = action_reply {\n            reply(value);\n        }\n"
EARLY = b"                if let Some((reply, value)) = action_reply.take() {\n                    reply(value);\n                }\n"
RECIPES = (
    ("M1", "lib", PRIVATE + "emit_callback_observes_unborrowed_owner",
     EMIT, b"                let session_borrow = self.session.borrow_mut();\n" + EMIT
     + b"                drop(session_borrow);\n", "owner borrow retained across emit"),
    ("M2", "lib", PRIVATE + "action_reply_callback_observes_unborrowed_owner",
     REPLY, b"        if let Some((reply, value)) = action_reply {\n"
     b"            let session_borrow = self.session.borrow_mut();\n            reply(value);\n"
     b"            drop(session_borrow);\n        }\n", "owner borrow retained across reply"),
    ("M3", "integration", CHANGED, COMMIT + EMIT, EARLY + COMMIT + EMIT,
     "reply preceded changed-view commit"),
    ("M4", "integration", CHANGED, COMMIT + EMIT, COMMIT + EARLY + EMIT,
     "reply preceded matching emit"),
)
EXPECTED_MUTANTS = {
    "M1": ("a24884587bc06bb952f0042c8ccc0f6691e8310b8161bcc65729846c10211267",
           "311b3451860cb98323ef85e0535392232212db6c2f17d9eb68b6eea0fff61f29",
           "src/platform/windows/code0_dispatcher.rs", 864),
    "M2": ("3b0b4fbe204025136b6380eb6095865ec4fc3904b8dcfac3fa1ce9b762ae6063",
           "50194d72a4f27b559b6e5d358bc63ea0f8f1152e85b2a162a456d30d20b3599a",
           "src/platform/windows/code0_dispatcher.rs", 864),
    "M3": ("4d1653295ce2ca5557303312ba9eb607378a8795cc50f111ebe39a0127aae385",
           "f5effdd5a8dad7a7d116982681f72b65426f545026cb86624f79fe9c56b1264f",
           "tests/code0_dispatcher.rs", 500),
    "M4": ("e2b0ae295ed6c36f9468726055d04a50eae9d26518790b1a8b297c2568c62de1",
           "35f3b03ca7d39b2c6632507a85905249e8b45793882bd31e445065341bb3b4e8",
           "tests/code0_dispatcher.rs", 510),
}
MAX_LOG = 4 * 1024 * 1024
MAX_ARCHIVE = 128 * 1024 * 1024
MAX_TREE = 256 * 1024 * 1024
MAX_FILES = 20000
TOTAL_SECONDS = 40 * 60
PHASE_SECONDS = 4 * 60
GIT_SECONDS = 30
FEATURE = "code0-native-fixture"
# Exact derived outputs written by tauri-build 2.6.3 / tauri-utils 2.9.3.
GENERATED = frozenset("windows/src-tauri/gen/schemas/" + name for name in (
    "acl-manifests.json", "capabilities.json", "windows-schema.json", "desktop-schema.json"))
SHA = re.compile(r"[0-9a-f]{40}")


class Incomplete(Exception):
    """Fixed diagnostic code, never arbitrary exception/profile/environment text."""


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def safe_path(path: Path, *, directory: bool = False) -> None:
    """Reject symlinks, Windows junctions/reparse points, and linked ancestors."""
    if not path.is_absolute():
        raise Incomplete("non_absolute_path")
    for parent in reversed(path.parents):
        info = parent.lstat()
        if (not stat.S_ISDIR(info.st_mode) or getattr(info, "st_file_attributes", 0)
                & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)):
            raise Incomplete("unsafe_path_ancestor")
    info = path.lstat()
    wanted = stat.S_ISDIR if directory else stat.S_ISREG
    if (not wanted(info.st_mode) or getattr(info, "st_file_attributes", 0)
            & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)):
        raise Incomplete("unsafe_path_leaf")


def bounded(path: Path, limit: int) -> bytes:
    safe_path(path)
    with path.open("rb") as stream:
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise Incomplete("evidence_overflow")
    return raw


def safe_name(name: str, *, directory: bool = False) -> str:
    path = PurePosixPath(name)
    if (not name or any(c in name for c in '\\:<>"|?*') or path.is_absolute()
            or any(p in ("", ".", "..") for p in name.split("/"))
            or any(p.lower() in (".git", "node_modules") for p in path.parts)
            or any(ord(c) < 32 for c in name)):
        raise Incomplete("unsafe_source_path")
    if any(p.lower() == ".superpowers" for p in path.parts):
        allowed = HISTORICAL_REPORT_DIRS if directory else HISTORICAL_REPORT_FILES
        if name not in allowed:
            raise Incomplete("unsafe_source_path")
    # Avoid Windows path aliases and device names during tar materialization.
    for part in path.parts:
        if part.endswith((".", " ")) or re.fullmatch(
                r"(?i)(con|prn|aux|nul|conin\$|conout\$|com[0-9¹²³]|lpt[0-9¹²³])(?:\..*)?", part):
            raise Incomplete("unsafe_windows_path")
    return name


def safe_archive_name(member: tarfile.TarInfo) -> str:
    if member.isdir():
        # Validate the parsed member name, not the original tar header spelling.
        # tarfile may already trim directory terminators. If still present here,
        # remove one; repeated parsed separators remain a lexical rejection.
        name = member.name[:-1] if member.name.endswith("/") else member.name
        return safe_name(name, directory=True)
    if not member.isfile():
        raise Incomplete("unsafe_archive_member")
    return safe_name(member.name)


def check_historical_paths(run: Run) -> None:
    # Pure fixed-name boundary oracle, still behind the hosted CI opt-in.
    for directory, names in ((False, HISTORICAL_REPORT_FILES), (True, HISTORICAL_REPORT_DIRS)):
        for name in names:
            run.check()
            if safe_name(name, directory=directory) != name:
                raise Incomplete("historical_path_oracle_rejected_input")
    denied = (
        (".superpowers", False),
        (".superpowers/sdd", False),
        (".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density", False),
        (".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/task-3-report.md", True),
        (".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/task-4-report.md", True),
        (".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/task-5-report.md", False),
        (".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/extra", True),
        (".superpowers/sdd/2026-10-26-code0-monitor/task-3-report.md", False),
        (".SUPERPOWERS/sdd/2026-09-04-windows-deepseek-history-and-density/task-3-report.md", False),
        (".superpowers/sdd/2026-09-04-windows-deepseek-history-and-density/task-3-report.md/extra", False),
        (".superpowers/sdd/../task-3-report.md", False),
        (".superpowers//sdd", True),
        (".superpowers/sdd/.", True),
        ("/.superpowers/sdd", True),
        (".superpowers/sdd ", True),
        (".git/config", False),
        ("node_modules/package.json", False),
        ("source/con.txt", False),
        ("source/file.", False),
        ("source/file ", False),
        ("source/file:stream", False),
        ("source\\file", False),
        ("source/\x00file", False),
    )
    for name, directory in denied:
        run.check()
        try:
            safe_name(name, directory=directory)
        except Incomplete:
            continue
        raise Incomplete("historical_path_oracle_missed_rejection")
    for name in HISTORICAL_REPORT_FILES:
        for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.FIFOTYPE):
            run.check()
            member = tarfile.TarInfo(name)
            member.type = kind
            try:
                safe_archive_name(member)
            except Incomplete:
                continue
            raise Incomplete("historical_path_oracle_missed_member_type")
    # These are direct parsed-name oracle inputs, not a raw-header parser test.
    for name in HISTORICAL_REPORT_DIRS:
        for suffix in ("", "/"):
            member = tarfile.TarInfo(name + suffix)
            member.type = tarfile.DIRTYPE
            if safe_archive_name(member) != name:
                raise Incomplete("historical_path_oracle_rejected_directory")
        member = tarfile.TarInfo(name + "//")
        member.type = tarfile.DIRTYPE
        try:
            safe_archive_name(member)
        except Incomplete:
            continue
        raise Incomplete("historical_path_oracle_missed_slash_alias")


def enumeration_error(_error: OSError) -> None:
    raise Incomplete("source_enumeration_failed")


def manifest(run: Run, root: Path) -> dict[str, str]:
    run.check()
    safe_path(root, directory=True)
    result = {}
    total = entries = 0
    for directory, dirs, files in os.walk(root, followlinks=False, onerror=enumeration_error):
        run.check()
        entries += len(dirs) + len(files)
        if entries > MAX_FILES:
            raise Incomplete("source_file_count_overflow")
        for name in dirs:
            run.check()
            relative = safe_name((Path(directory) / name).relative_to(root).as_posix(), directory=True)
            safe_path(root / relative, directory=True)
        for name in files:
            path = Path(directory) / name
            relative = safe_name(path.relative_to(root).as_posix())
            raw = run.read(path, MAX_TREE)
            total += len(raw)
            if total > MAX_TREE:
                raise Incomplete("source_tree_overflow")
            result[relative] = digest(raw)
    run.check()
    return result


def unchanged(run: Run, copy: Path, expected: dict[str, str]) -> list[dict]:
    actual = manifest(run, copy)
    # Only the four proven derived outputs may be added/rewritten; never deleted.
    if any(actual.get(name) != sha for name, sha in expected.items() if name not in GENERATED):
        raise Incomplete("copied_source_drift")
    if set(actual) - set(expected) - GENERATED or set(expected) - set(actual):
        raise Incomplete("unexpected_source_addition_or_deletion")
    outputs = []
    for name in sorted(GENERATED):
        run.check()
        validate_json_object(run.read(copy / name, MAX_LOG))
        outputs.append({"name": name, "copyBeforeSha256": expected.get(name),
                        "compiledAfterSha256": actual.get(name)})
    run.check()
    return outputs


def validate_json_object(raw: bytes) -> None:
    def invalid_constant(_):
        raise Incomplete("non_json_constant")

    def unique_object(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise Incomplete("duplicate_json_key")
            value[key] = item
        return value

    value = json.loads(raw.decode("utf-8"), parse_constant=invalid_constant,
                       object_pairs_hook=unique_object)
    if not isinstance(value, dict):
        raise Incomplete("generated_json_invalid")


class Run:
    def __init__(self, output: Path, deadline: float):
        self.output = output
        self.deadline = deadline
        self.unreaped = False
        self.artifacts = []
        self.env = os.environ.copy()
        # No inherited product overrides, injected Rust flags/wrappers, or native fixture mode.
        for key in list(self.env):
            if key.startswith(("AI_METER_", "CODE0_")) or key in (
                    "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                    "RUSTC_WORKSPACE_WRAPPER", "RUST_TEST_THREADS", "RUST_TEST_NOCAPTURE"):
                self.env.pop(key)
        self.env.update(TEMP=str(output / "tmp"), TMP=str(output / "tmp"),
                        CARGO_NET_OFFLINE="true", CARGO_TERM_COLOR="never",
                        CARGO_TARGET_DIR=str(output / "target"), RUSTUP_TOOLCHAIN="1.88.0",
                        RUSTUP_AUTO_INSTALL="0", RUST_BACKTRACE="0")

    def check(self) -> None:
        if time.monotonic() >= self.deadline:
            raise Incomplete("total_deadline")

    def read(self, path: Path, limit: int) -> bytes:
        self.check()
        raw = bounded(path, limit)
        self.check()
        return raw

    def remove(self, path: Path, *, directory: bool = True) -> None:
        self.check()
        if self.unreaped:
            raise Incomplete("cleanup_refused_active_process")
        safe_path(path, directory=directory)
        if directory:
            shutil.rmtree(path)
        else:
            path.unlink()
        self.check()

    def write_artifact(self, name: str, raw: bytes) -> None:
        self.check()
        with (self.output / name).open("xb") as stream:
            stream.write(raw)
        self.artifacts.append({"name": name, "complete": True, "sha256": digest(raw)})
        self.check()

    def command(self, name: str, argv: list[str], cwd: Path,
                *, binary: bool = False, limit: int = MAX_LOG,
                seconds: int = PHASE_SECONDS) -> tuple[int, bytes]:
        self.check()
        safe_path(cwd, directory=True)
        path = self.output / name
        err = self.output / (name + ".stderr") if binary else None
        evidence = [{"name": name, "complete": False, "sha256": None}]
        if err is not None:
            evidence.append({"name": err.name, "complete": False, "sha256": None})
        self.artifacts.extend(evidence)
        end = min(self.deadline, time.monotonic() + seconds)
        # Fixed private files; never PIPE/capture_output, shell, timeout-kill, or signals.
        with path.open("xb") as stdout:
            with (err.open("xb") if err else open(os.devnull, "wb")) as stderr:
                child = subprocess.Popen(argv, cwd=cwd, env=self.env, stdin=subprocess.DEVNULL,
                                         stdout=stdout, stderr=stderr if binary else subprocess.STDOUT,
                                         shell=False)
                self.unreaped = True
                while child.poll() is None:
                    if (time.monotonic() >= end or path.stat().st_size > limit
                            or (err and err.stat().st_size > MAX_LOG)):
                        # Never read/digest active files, signal, clean up, or start another child.
                        raise Incomplete("active_process_deadline_or_overflow")
                    time.sleep(0.1)
                code = child.wait()  # Already observed exited; explicitly reap normal paths.
                self.unreaped = False
        if time.monotonic() >= end:
            raise Incomplete("phase_finished_after_deadline")
        raw = self.read(path, limit)
        evidence[0].update(complete=True, sha256=digest(raw))
        if err:
            error_raw = self.read(err, MAX_LOG)
            evidence[1].update(complete=True, sha256=digest(error_raw))
        self.check()
        if time.monotonic() >= end:
            raise Incomplete("phase_evidence_deadline")
        return code, raw

    def cargo(self, label: str, copy: Path, args: list[str]) -> tuple[int, str]:
        code, raw = self.command(label + ".log", ["cargo", *args], copy / "windows/src-tauri")
        try:
            return code, raw.decode("utf-8")
        except UnicodeError:
            raise Incomplete("non_utf8_cargo_evidence") from None


def check_suite(code: int, log: str, count: int, *, target: str | None = None,
                assertion: str | None = None, location: tuple[str, int] | None = None) -> None:
    lines = log.replace("\\", "/").splitlines()
    wanted = "FAILED" if assertion else "ok"
    if [x for x in lines if re.fullmatch(r"running \d+ tests?", x)] != [
            f"running {count} test" + ("s" if count != 1 else "")]:
        raise Incomplete("selected_count_mismatch")
    expected_code = 101 if assertion else 0
    if code != expected_code:
        raise Incomplete("test_exit_mismatch")
    summaries = [x for x in lines if x.startswith("test result:")]
    passed, failed = (0, 1) if assertion else (count, 0)
    summary = (rf"test result: {wanted}\. {passed} passed; {failed} failed; "
               rf"0 ignored; 0 measured; \d+ filtered out; finished in .+")
    if len(summaries) != 1 or re.fullmatch(summary, summaries[0]) is None:
        raise Incomplete("test_summary_mismatch")
    if target:
        if lines.count(f"test {target} ... {wanted}") != 1:
            raise Incomplete("target_result_mismatch")
    if assertion:
        file, line = location
        header = f"thread '{target}' panicked at {file}:{line}:"
        panics = [i for i, x in enumerate(lines) if x.startswith("thread '") and "panicked at" in x]
        if len(panics) != 1 or not lines[panics[0]].startswith(header):
            raise Incomplete("semantic_panic_location_mismatch")
        if panics[0] + 1 >= len(lines) or lines[panics[0] + 1] not in (
                assertion, f"assertion `left == right` failed: {assertion}"):
            raise Incomplete("semantic_assertion_mismatch")


def baseline_tree(run: Run, checkout: Path, summary: dict) -> dict[str, bytes]:
    code, raw = run.command("checkout-sha.log", ["git", "rev-parse", "--verify", "HEAD"],
                            checkout, seconds=GIT_SECONDS)
    actual = raw.decode("ascii").strip()
    if code != 0 or SHA.fullmatch(actual) is None:
        raise Incomplete("checkout_sha_invalid")
    summary["checkoutSha"] = actual
    run.check()
    code, _ = run.command("source.tar", ["git", "archive", "--format=tar", actual], checkout,
                          binary=True, limit=MAX_ARCHIVE, seconds=GIT_SECONDS)
    if code:
        raise Incomplete("source_archive_failed")
    files = {}
    names = set()
    total = 0
    with tarfile.open(run.output / "source.tar", "r:") as archive:
        for member in archive:
            run.check()
            name = safe_archive_name(member)
            folded = name.casefold()
            if folded in names:
                raise Incomplete("duplicate_archive_path")
            names.add(folded)
            if len(names) > MAX_FILES:
                raise Incomplete("source_file_count_overflow")
            if member.isdir():
                safe_path(checkout / name, directory=True)
                continue
            if not member.isfile() or not 0 <= member.size <= MAX_TREE:
                raise Incomplete("unsafe_archive_member")
            total += member.size
            if total > MAX_TREE:
                raise Incomplete("archive_expansion_overflow")
            with archive.extractfile(member) as stream:
                data = stream.read(member.size + 1)
            if len(data) != member.size:
                raise Incomplete("archive_member_incomplete")
            # Reject dirty tracked inputs rather than silently using another source context.
            checkout_data = run.read(checkout / name, MAX_TREE)
            if name not in GENERATED and checkout_data != data:
                raise Incomplete("tracked_checkout_differs_from_head")
            files[name] = data
    if not HISTORICAL_REPORT_FILES.issubset(files) or not HISTORICAL_REPORT_DIRS.issubset(names):
        raise Incomplete("historical_archive_entries_missing")
    if digest(files.get(DISPATCHER, b"")) != DISPATCHER_SHA or digest(files.get(TESTS, b"")) != TESTS_SHA:
        raise Incomplete("unaccepted_source_hash")
    for name in sorted(GENERATED):
        run.check()
        original = run.read(checkout / name, MAX_LOG)
        validate_json_object(original)
        head = files.get(name)
        if head is not None:
            validate_json_object(head)
        summary["generatedInputs"].append({"name": name,
            "headSha256": digest(head) if head is not None else None,
            "checkoutSha256": digest(original),
            "copyBeforeSha256": digest(head) if head is not None else None})
    dist = checkout / "windows/dist"
    safe_path(dist, directory=True)
    for name in manifest(run, dist):
        run.check()
        full = "windows/dist/" + name
        data = run.read(dist / name, MAX_TREE)
        if full in files and files[full] != data:
            raise Incomplete("tracked_dist_conflict")
        files[full] = data
    if "windows/dist/index.html" not in files or sum(map(len, files.values())) > MAX_TREE:
        raise Incomplete("generated_dist_incomplete_or_overflow")
    run.check()
    return files


def fresh_copy(run: Run, name: str, files: dict[str, bytes]) -> Path:
    run.check()
    copy = run.output / name
    copy.mkdir()
    for relative, data in files.items():
        run.check()
        path = copy / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    run.check()
    return copy


def compile_copy(run: Run, name: str, copy: Path, result: dict,
                 expected: dict[str, str]) -> None:
    for phase, args in (
        ("clippy", ["clippy", "--locked", "--offline", "--all-targets", "--features", FEATURE, "--", "-D", "warnings"]),
        ("compile", ["test", "--locked", "--offline", "--features", FEATURE, "--lib", "--test", "code0_dispatcher", "--no-run"]),
    ):
        result[phase] = "started"
        code, _ = run.cargo(name + "-" + phase, copy, args)
        result[phase] = "passed" if code == 0 else "failed"
        if code != 0:
            raise Incomplete("strict_compile_precondition_failed")
        result["generatedOutputs"] = unchanged(run, copy, expected)


def check_enumeration_error(run: Run) -> None:
    # CI-only deterministic regression: readable index, denied assets enumeration.
    run.check()
    fixture = run.output / "enumeration-oracle"
    assets = fixture / "assets"
    assets.mkdir(parents=True)
    (fixture / "index.html").write_bytes(b"synthetic index")
    (assets / "fixture.txt").write_bytes(b"synthetic asset")
    expected = {"index.html": digest(b"synthetic index"),
                "assets/fixture.txt": digest(b"synthetic asset")}
    if manifest(run, fixture) != expected:
        raise Incomplete("enumeration_oracle_fixture_incomplete")
    original_scandir = os.scandir

    def denied_scandir(path):
        if Path(path) == assets:
            raise PermissionError("synthetic enumeration failure")
        return original_scandir(path)

    try:
        os.scandir = denied_scandir
        try:
            manifest(run, fixture)
        except Incomplete as error:
            if str(error) != "source_enumeration_failed":
                raise
        else:
            raise Incomplete("enumeration_error_oracle_missed")
    finally:
        os.scandir = original_scandir
    run.remove(fixture)


def execute(run: Run, checkout: Path, summary: dict) -> None:
    summary["historicalPathOracle"] = "started"
    check_historical_paths(run)
    summary["historicalPathOracle"] = "passed"
    summary["enumerationErrorOracle"] = "started"
    check_enumeration_error(run)
    summary["enumerationErrorOracle"] = "passed"
    files = baseline_tree(run, checkout, summary)
    baseline = {name: digest(data) for name, data in files.items()}
    summary["baselineManifestSha256"] = digest(json.dumps(baseline, sort_keys=True).encode())
    source = files[DISPATCHER]
    copy = fresh_copy(run, "baseline", files)
    compile_copy(run, "baseline", copy, summary["baseline"], baseline)
    for label, args, count in (
        ("integration", ["--test", "code0_dispatcher"], 23),
        ("private", ["--lib", PRIVATE], 2),
    ):
        summary["baseline"][label] = "started"
        code, log = run.cargo("baseline-" + label, copy,
                             ["test", "--locked", "--offline", "--features", FEATURE, *args, "--", "--test-threads=1"])
        check_suite(code, log, count)
        summary["baseline"][label] = "passed"
        key = "validatedIntegrationTests" if label == "integration" else "validatedPrivateTests"
        summary["baseline"][key] = count
        summary["baseline"]["generatedOutputs"] = unchanged(run, copy, baseline)
    summary["baseline"]["generatedOutputs"] = unchanged(run, copy, baseline)
    run.remove(copy)
    summary["baseline"]["cleanup"] = "passed"
    summary["baselineGreen"] = True
    for recipe, result in zip(RECIPES, summary["mutants"], strict=True):
        run.check()
        result["status"] = "invalid"
        name, kind, target, before, after, assertion = recipe
        if source.count(before) != 1:
            raise Incomplete("mutation_anchor_mismatch")
        mutated = source.replace(before, after, 1)
        expected = dict(baseline, **{DISPATCHER: digest(mutated)})
        result["beforeSha256"] = DISPATCHER_SHA
        after_sha, patch_sha, panic_file, panic_line = EXPECTED_MUTANTS[name]
        if digest(mutated) != after_sha:
            raise Incomplete("mutation_hash_mismatch")
        result["afterSha256"] = after_sha
        patch = "".join(difflib.unified_diff(source.decode().splitlines(True),
                       mutated.decode().splitlines(True), fromfile="a/" + DISPATCHER,
                       tofile="b/" + DISPATCHER)).encode()
        if digest(patch) != patch_sha:
            raise Incomplete("mutation_patch_hash_mismatch")
        run.write_artifact(name + ".patch", patch)
        result["patchSha256"] = digest(patch)
        copy = fresh_copy(run, name, files)
        (copy / DISPATCHER).write_bytes(mutated)
        if manifest(run, copy) != expected:
            raise Incomplete("mutation_source_delta_mismatch")
        compile_copy(run, name, copy, result, expected)
        if result["generatedOutputs"] != summary["baseline"]["generatedOutputs"]:
            raise Incomplete("mutant_generated_outputs_differ_from_baseline")
        location = (panic_file, panic_line)
        selection = ["--lib"] if kind == "lib" else ["--test", "code0_dispatcher"]
        result["test"] = "started"
        code, log = run.cargo(name + "-test", copy,
                             ["test", "--locked", "--offline", "--features", FEATURE, *selection, target,
                              "--", "--exact", "--test-threads=1"])
        result["test"] = "survived" if code == 0 else "invalid_failure"
        if code == 0:
            result["status"] = "survived"
        check_suite(code, log, 1, target=target, assertion=assertion, location=location)
        result["validatedSelectedTests"] = result["validatedFailedTests"] = 1
        result["test"] = "semantic_failure"
        result["semanticAssertionMatch"] = True
        result["generatedOutputs"] = unchanged(run, copy, expected)
        if result["generatedOutputs"] != summary["baseline"]["generatedOutputs"]:
            raise Incomplete("post_test_generated_outputs_differ_from_baseline")
        run.remove(copy)
        result["cleanup"] = "passed"
        result["status"] = "killed"
    # Only disposable source/build artifacts are removed; evidence logs/patches survive.
    run.remove(run.output / "target")
    run.remove(run.output / "tmp")
    run.remove(run.output / "source.tar", directory=False)
    run.artifacts = [item for item in run.artifacts if item["name"] != "source.tar"]
    run.check()
    summary["cleanup"] = "passed"
    summary["status"] = "passed"


def main() -> int:
    deadline = time.monotonic() + TOTAL_SECONDS
    if (os.name != "nt" or sys.platform != "win32" or os.environ.get("GITHUB_ACTIONS") != "true"
            or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted"
            or not os.environ.get("GITHUB_RUN_ID", "").isdigit()
            or os.environ.get("CODE0_DISPATCHER_MUTATIONS_APPROVED") != "otherwise-GREEN25"):
        print("Dispatcher mutations refused: approved ephemeral Windows GitHub CI only.")
        return 2
    # No argument or arbitrary command interface. Output is fresh, under runner-owned temp only.
    if len(sys.argv) != 1:
        return 2
    checkout = Path(os.path.abspath(__file__)).parent.parent
    try:
        safe_path(Path(os.path.abspath(__file__)))
        if checkout != Path(os.environ["GITHUB_WORKSPACE"]):
            raise Incomplete("outside_github_workspace")
        temporary = Path(os.environ["RUNNER_TEMP"])
        safe_path(temporary, directory=True)
    except (Incomplete, OSError, KeyError):
        print("Dispatcher mutations refused: unsafe CI paths.")
        return 2
    try:
        output = Path(tempfile.mkdtemp(prefix="code0-dispatcher-mutations-", dir=temporary))
        os.chmod(output, 0o700)
    except OSError:
        print("Dispatcher mutation output initialization failed; evidence incomplete.")
        return 1
    summary = {
        "schema": 1, "status": "failed", "error": "none", "checkoutSha": None,
        "feature": FEATURE, "limits": {"totalSeconds": TOTAL_SECONDS,
            "cargoPhaseSeconds": PHASE_SECONDS, "gitPhaseSeconds": GIT_SECONDS,
            "textLogBytes": MAX_LOG, "archiveBytes": MAX_ARCHIVE,
            "expandedTreeBytes": MAX_TREE, "treeEntries": MAX_FILES},
        "githubSha": None, "prHeadSha": None, "prMergeSha": None,
        "dispatcherSha256": DISPATCHER_SHA, "testsSha256": TESTS_SHA,
        "baselineManifestSha256": None, "baselineGreen": False, "generatedInputs": [],
        "baseline": dict(dict.fromkeys(("clippy", "compile", "integration", "private", "cleanup"), "unrun"), generatedOutputs=[],
                         requiredIntegrationTests=23, requiredPrivateTests=2,
                         validatedIntegrationTests=0, validatedPrivateTests=0),
        "mutants": [{"id": r[0], "target": r[2], "assertion": r[5],
                     "beforeSha256": None, "afterSha256": None, "patchSha256": None,
                     "clippy": "unrun", "compile": "unrun", "test": "unrun",
                     "semanticAssertionMatch": False, "cleanup": "unrun", "status": "unrun",
                     "generatedOutputs": [], "requiredTests": 1,
                     "validatedSelectedTests": 0, "validatedFailedTests": 0}
                    for r in RECIPES],
        "cleanup": "unrun", "processUnreaped": False, "artifacts": [],
        "enumerationErrorOracle": "unrun",
        "historicalPathOracle": "unrun",
    }
    run = Run(output, deadline)
    try:
        run.check()
        (output / "tmp").mkdir()
        value = os.environ.get("GITHUB_SHA", "")
        if SHA.fullmatch(value):
            summary["githubSha"] = value
        event = json.loads(run.read(Path(os.environ["GITHUB_EVENT_PATH"]), 1024 * 1024))
        pr = event.get("pull_request", {})
        for field, value in (("prHeadSha", pr.get("head", {}).get("sha")),
                             ("prMergeSha", pr.get("merge_commit_sha"))):
            if isinstance(value, str) and SHA.fullmatch(value):
                summary[field] = value
        execute(run, checkout, summary)
    except Incomplete as error:
        summary["error"] = str(error)
    except Exception:
        summary["error"] = "unexpected_error_evidence_incomplete"
    finally:
        summary["processUnreaped"] = run.unreaped
        # Only immutable in-memory records of previously completed evidence are used here.
        # Active/missing/overflowed files retain complete=false and sha256=null, never reread.
        summary["artifacts"] = run.artifacts
        try:
            run.check()
        except Incomplete:
            summary.update(status="failed", error="total_deadline")
        if summary["status"] != "passed":
            summary["cleanup"] = "preserved_after_failure"
        try:
            destination = output / "summary.json"
            destination.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
            # Check even final write completion; a late summary cannot turn into success.
            if time.monotonic() >= run.deadline and summary["status"] == "passed":
                summary.update(status="failed", error="finalization_deadline")
                destination.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
        except Exception:
            print("Dispatcher mutation summary finalization failed; evidence incomplete.")
            return 1
        print("Dispatcher mutation evidence: " + output.name + "/summary.json")
    return 0 if summary["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
