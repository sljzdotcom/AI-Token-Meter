import Foundation

enum GeminiLoginResult: String, Equatable {
    case success
    case failure
    case cancelled
    case timedOut = "timeout"
}

struct GeminiLoginReceipt: Equatable {
    let token: String
    let result: GeminiLoginResult

    init?(url: URL) {
        guard url.scheme?.lowercased() == "aitokenmeter",
              url.host?.lowercased() == "antigravity-login-complete",
              let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let items = components.queryItems,
              items.count == 3,
              Set(items.map(\.name)) == ["token", "result", "exit_code"],
              let token = items.first(where: { $0.name == "token" })?.value,
              UUID(uuidString: token) != nil,
              let rawResult = items.first(where: { $0.name == "result" })?.value,
              let result = GeminiLoginResult(rawValue: rawResult),
              let rawExitCode = items.first(where: { $0.name == "exit_code" })?.value,
              let exitCode = Int(rawExitCode), (0...255).contains(exitCode) else { return nil }

        switch result {
        case .success where exitCode == 0:
            break
        case .failure where exitCode != 0:
            break
        case .cancelled where [129, 130, 143].contains(exitCode):
            break
        case .timedOut where exitCode == 124:
            break
        default:
            return nil
        }

        self.token = token
        self.result = result
    }
}
