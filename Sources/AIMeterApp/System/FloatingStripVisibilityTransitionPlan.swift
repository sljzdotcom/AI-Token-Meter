import Foundation

enum FloatingStripVisibilityDestination {
    case folded
    case expanded
}

struct FloatingStripVisibilityTransitionPlan {
    let contentDelay: Duration

    static func make(
        destination: FloatingStripVisibilityDestination,
        reduceMotion: Bool
    ) -> Self {
        let contentDelay: Duration
        if reduceMotion {
            contentDelay = .zero
        } else {
            contentDelay = switch destination {
            case .folded: .milliseconds(140)
            case .expanded: .milliseconds(180)
            }
        }
        return Self(
            contentDelay: contentDelay
        )
    }
}
