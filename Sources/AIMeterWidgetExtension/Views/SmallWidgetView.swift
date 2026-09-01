import AIMeterCore
import SwiftUI

struct SmallWidgetView: View {
    let providers: [WidgetProviderSnapshot]

    var body: some View {
        HStack(spacing: 8) {
            ForEach(providers, id: \.provider) { snapshot in
                ZStack {
                    RoundedRectangle(cornerRadius: 15, style: .continuous)
                        .fill(Color.black.opacity(0.34))
                    WidgetProgressRing(snapshot: snapshot, lineWidth: 4)
                        .padding(5)
                    WidgetProviderLogo(provider: snapshot.provider)
                        .frame(width: 19, height: 19)
                }
                .aspectRatio(1, contentMode: .fit)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(snapshot.provider.name + ", " + snapshot.valueText)
            }
        }
        .padding(12)
    }
}
