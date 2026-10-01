import Darwin
import Foundation
import Testing
@testable import AIMeterCore

@Suite("Process-group command cleanup", .serialized)
struct ProcessGroupCommandRunnerTests {
    @Test("Cancellation terminates a forked child in the owned process group")
    func cancellationTerminatesOwnedDescendant() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("AI-Meter-ProcessGroup-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let ready = directory.appendingPathComponent("ready")
        let executable = directory.appendingPathComponent("child-fixture")
        try compileFixture(to: executable)
        let unrelated = Process()
        unrelated.executableURL = URL(fileURLWithPath: "/bin/sleep")
        unrelated.arguments = ["30"]
        try unrelated.run()
        defer {
            if unrelated.isRunning { unrelated.terminate() }
            unrelated.waitUntilExit()
        }

        let task = Task {
            try await ProcessGroupCommandRunner().run(CommandRequest(
                executableURL: executable,
                arguments: [ready.path],
                inputLines: [],
                timeout: 5
            ))
        }
        for _ in 0..<200 where !FileManager.default.fileExists(atPath: ready.path) {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(FileManager.default.fileExists(atPath: ready.path))
        task.cancel()
        await #expect(throws: CancellationError.self) { try await task.value }
        #expect(unrelated.isRunning)

        let childPID = try #require(pid_t(String(contentsOf: ready, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)))
        defer { _ = kill(childPID, SIGKILL) }
        for _ in 0..<100 where kill(childPID, 0) == 0 {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(kill(childPID, 0) == -1)
    }

    private func compileFixture(to executable: URL) throws {
        let source = """
        #include <fcntl.h>
        #include <stdio.h>
        #include <unistd.h>
        int main(int argc, char **argv) {
          if (argc != 2) return 2;
          pid_t child = fork();
          if (child < 0) return 3;
          if (child == 0) for (;;) pause();
          int fd = open(argv[1], O_WRONLY | O_CREAT | O_TRUNC, 0600);
          if (fd >= 0) { (void)dprintf(fd, "%d", child); (void)close(fd); }
          for (;;) pause();
        }
        """
        let compiler = Process()
        let input = Pipe()
        compiler.executableURL = URL(fileURLWithPath: "/usr/bin/clang")
        compiler.arguments = ["-x", "c", "-", "-o", executable.path]
        compiler.standardInput = input.fileHandleForReading
        compiler.standardOutput = FileHandle.nullDevice
        compiler.standardError = FileHandle.nullDevice
        try compiler.run()
        try input.fileHandleForWriting.write(contentsOf: Data(source.utf8))
        try input.fileHandleForWriting.close()
        compiler.waitUntilExit()
        #expect(compiler.terminationStatus == 0)
    }
}
