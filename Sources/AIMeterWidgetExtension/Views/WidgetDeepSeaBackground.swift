import SwiftUI

struct WidgetDeepSeaBackground: View {
    var body: some View {
        ZStack {
            Color(red: 0.015, green: 0.03, blue: 0.075)
            if let image = WidgetResource.image(
                name: "floating-strip-deep-sea",
                extension: "png",
                subdirectory: "Backgrounds"
            ) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFill()
                    .opacity(0.72)
            }
            LinearGradient(
                colors: [
                    Color.black.opacity(0.05),
                    Color(red: 0.01, green: 0.025, blue: 0.07).opacity(0.55),
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
        }
    }
}
