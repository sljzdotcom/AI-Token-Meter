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
                            colors: snapshot.provider.accentColors,
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
