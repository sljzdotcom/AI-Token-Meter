import AIMeterCore
import SwiftUI

struct AIMeterProviderPalette: Hashable {
    let startHex: UInt32
    let endHex: UInt32

    var startColor: Color {
        Color(hex: startHex)
    }

    var endColor: Color {
        Color(hex: endHex)
    }

    var gradient: LinearGradient {
        LinearGradient(
            colors: [startColor, endColor],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }
}

enum AIMeterAccentRole: Equatable {
    case provider(UsageProvider)
    case semantic(UsageSemantic)
}

extension UsageProvider {
    var accentPalette: AIMeterProviderPalette {
        switch self {
        case .claude:
            .init(startHex: 0xE8B96D, endHex: 0xD97757)
        case .codex:
            .init(startHex: 0xFF6FAE, endHex: 0xA96DFF)
        case .deepSeek:
            .init(startHex: 0x54EDC6, endHex: 0x7769FF)
        }
    }
}

extension UsageSemantic {
    func accentRole(for provider: UsageProvider) -> AIMeterAccentRole {
        self == .normal ? .provider(provider) : .semantic(self)
    }

    func accentStyle(for provider: UsageProvider) -> AnyShapeStyle {
        switch accentRole(for: provider) {
        case .provider(let provider):
            AnyShapeStyle(provider.accentPalette.gradient)
        case .semantic(let semantic):
            AnyShapeStyle(semantic.color)
        }
    }
}

private extension Color {
    init(hex: UInt32) {
        self.init(
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
    }
}
