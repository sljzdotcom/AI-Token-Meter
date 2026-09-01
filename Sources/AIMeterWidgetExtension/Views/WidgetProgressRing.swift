import AIMeterCore
import SwiftUI

struct WidgetProgressRing: View {
    let snapshot: WidgetProviderSnapshot
    var lineWidth: CGFloat = 5

    var body: some View {
        ZStack {
            Circle()
                .stroke(Color.white.opacity(0.12), lineWidth: lineWidth)
            if let fraction = snapshot.fraction {
                Circle()
                    .trim(from: 0, to: max(fraction, 0.018))
                    .stroke(
                        AngularGradient(
                            colors: snapshot.semantic.accentColors(for: snapshot.provider),
                            center: .center,
                            startAngle: .degrees(-90),
                            endAngle: .degrees(270)
                        ),
                        style: StrokeStyle(lineWidth: lineWidth, lineCap: .round)
                    )
                    .rotationEffect(.degrees(-90))
            } else {
                Circle()
                    .trim(from: 0, to: 0.035)
                    .stroke(Color.white.opacity(0.28), style: StrokeStyle(lineWidth: lineWidth, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
        }
    }
}

struct WidgetStatusIndicator: View {
    let semantic: WidgetSnapshotSemantic

    @ViewBuilder var body: some View {
        if let symbol = semantic.statusSymbol {
            Image(systemName: symbol)
                .font(.system(size: 8, weight: .bold))
                .foregroundStyle(.white)
                .padding(3)
                .background(Color.black.opacity(0.72), in: Circle())
                .accessibilityHidden(true)
        }
    }
}

extension WidgetSnapshotSemantic {
    var statusSymbol: String? {
        switch self {
        case .normal: nil
        case .warning: "exclamationmark.triangle.fill"
        case .critical: "exclamationmark.octagon.fill"
        case .stale: "clock.badge.exclamationmark"
        case .unavailable: "slash.circle.fill"
        }
    }

    var accessibilityText: String {
        switch self {
        case .normal: "Available"
        case .warning: "Warning"
        case .critical: "Critical"
        case .stale: "Cached"
        case .unavailable: "Unavailable"
        }
    }
}
