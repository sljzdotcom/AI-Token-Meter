import AIMeterCore
import Observation

@MainActor
@Observable
final class AppModel {
    private(set) var snapshots: [UsageSnapshot] = []
}

