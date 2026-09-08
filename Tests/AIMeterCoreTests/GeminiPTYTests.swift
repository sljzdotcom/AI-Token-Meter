import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini PTY lifecycle", .serialized)
struct GeminiPTYTests {
    @Test func waitsForReadyTypesOnlyFixedCommandsAndExits() async throws {
        let fixture = try makeFixture(mode: "ready")
        defer { try? FileManager.default.removeItem(at: fixture.directory) }
        let result = try await PTYCommandRunner().run(fixture.request)
        #expect(result.exitCode == 0)
        #expect(try GeminiUsageParser().parse(result.output).primaryMetric?.current == 60)
        let inputs = try String(contentsOf: fixture.directory.appendingPathComponent("input"), encoding: .utf8)
        #expect(inputs == "/model\r\u{1b}/quit\r")
        #expect(!result.output.contains("WRONG_ENV"))
    }
    @Test func authenticationStopsBeforeAnyInputAndReapsProcess() async throws {
        let fixture = try makeFixture(mode: "auth")
        defer { try? FileManager.default.removeItem(at: fixture.directory) }
        await #expect(throws: UsageCollectionError.authenticationRequired) { try await PTYCommandRunner().run(fixture.request) }
        #expect((try String(contentsOf: fixture.directory.appendingPathComponent("input"), encoding: .utf8)).isEmpty)
        try assertExited(fixture.directory)
    }
    @Test func unknownDialogStopsWithoutSelectingAnything() async throws {
        let fixture = try makeFixture(mode: "theme")
        defer { try? FileManager.default.removeItem(at: fixture.directory) }
        await #expect(throws: (any Error).self) { try await PTYCommandRunner().run(fixture.request) }
        #expect((try String(contentsOf: fixture.directory.appendingPathComponent("input"), encoding: .utf8)).isEmpty)
        try assertExited(fixture.directory)
    }
    @Test func cancellationAndTimeoutReapTheChild() async throws {
        for cancel in [false, true] {
            let fixture = try makeFixture(mode: "hang", timeout: cancel ? 5 : 0.3)
            defer { try? FileManager.default.removeItem(at: fixture.directory) }
            let started = Date()
            let task = Task { try await PTYCommandRunner().run(fixture.request) }
            if cancel { try await Task.sleep(for: .milliseconds(200)); task.cancel() }
            do { _ = try await task.value; Issue.record("Expected bounded termination") } catch {}
            #expect(Date().timeIntervalSince(started) < 2)
            try assertExited(fixture.directory)
        }
    }
    @Test func conflictingCompleteFramesCannotBecomeFreshQuota() async throws {
        let fixture = try makeFixture(mode: "conflict")
        defer { try? FileManager.default.removeItem(at: fixture.directory) }
        await #expect(throws: UsageCollectionError.unrecognizedOutput) { try await PTYCommandRunner().run(fixture.request) }
        try assertExited(fixture.directory)
    }
    @Test(arguments: ["tail-conflict", "tail-auth", "tail-repeat"])
    func finalWriteIsValidatedBeforeReturningCapturedQuota(_ mode: String) async throws {
        let fixture = try makeFixture(mode: mode)
        defer { try? FileManager.default.removeItem(at: fixture.directory) }
        if mode == "tail-repeat" {
            let result = try await PTYCommandRunner().run(fixture.request)
            #expect(try GeminiUsageParser().parse(result.output).primaryMetric?.current == 60)
        } else {
            let error: UsageCollectionError = mode == "tail-auth" ? .authenticationRequired : .unrecognizedOutput
            await #expect(throws: error) { try await PTYCommandRunner().run(fixture.request) }
        }
        try assertExited(fixture.directory)
    }
    private func assertExited(_ directory: URL) throws {
        let pid = try #require(Int32(String(contentsOf: directory.appendingPathComponent("pid"), encoding: .utf8)))
        #expect(kill(pid, 0) == -1)
    }
    private func makeFixture(mode: String, timeout: Double = 6) throws -> (directory: URL, request: CommandRequest) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-pty-\(UUID())")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let script = #"""
import os,sys,time,tty
from pathlib import Path
tty.setraw(sys.stdin.fileno())
root=Path(sys.argv[1]); (root/'pid').write_text(str(os.getpid())); (root/'input').write_text('')
mode=sys.argv[2]
if os.environ.get('GEMINI_TEST_ENV')!='isolated': print('WRONG_ENV',flush=True)
if mode=='auth': print('Enter the authorization code:',flush=True)
elif mode=='theme': print('Select a theme',flush=True)
elif mode in ('ready','conflict','tail-conflict','tail-auth','tail-repeat'):
 time.sleep(.2); print('? for shortcuts\r\n> Type your message or @path/to/file',flush=True)
buf=b''
while True:
 c=os.read(0,1)
 if not c: break
 with (root/'input').open('ab') as f: f.write(c)
 buf+=c
 if buf==b'/model\r':
  print('\x1b[2J\x1b[H╭────────────╮\r\nSelect Model\r\nModel usage\r\nPro 25%\r\nFlash 60%\r\n(Press Esc to close)\r\n╰────────────╯',flush=True)
  if mode=='conflict':
   time.sleep(.12); print('\x1b[2J\x1b[H╭────────────╮\r\nSelect Model\r\nModel usage\r\nPro 30%\r\nFlash 60%\r\n(Press Esc to close)\r\n╰────────────╯',flush=True)
 elif buf==b'/model\r\x1b/quit\r':
  if mode=='tail-auth': tail='\x1b[2J\x1b[HEnter the authorization code:\r\n'
  elif mode in ('tail-conflict','tail-repeat'):
   value=30 if mode=='tail-conflict' else 25
   tail=f'\x1b[2J\x1b[H╭────────────╮\r\nSelect Model\r\nModel usage\r\nPro {value}%\r\nFlash 60%\r\n(Press Esc to close)\r\n╰────────────╯\r\n'
  else: tail=''
  os.write(1,(tail+'\x1b[2J\x1b[HGoodbye').encode()); os._exit(0)
"""#
        return (directory, CommandRequest(executableURL: URL(fileURLWithPath: "/usr/bin/python3"), arguments: ["-u", "-c", script, directory.path, mode], inputLines: [], timeout: timeout, currentDirectoryURL: directory, environment: ["PATH": "/usr/bin:/bin", "GEMINI_TEST_ENV": "isolated"], geminiQuotaInteraction: true))
    }
}
