import Foundation

enum AppResourceLocator {
    static func url(
        forResource name: String,
        withExtension fileExtension: String,
        subdirectory: String? = nil,
        primaryBundle: Bundle = .main,
        packageBundle: () -> Bundle? = { Bundle.module }
    ) -> URL? {
        if let url = primaryBundle.url(
            forResource: name,
            withExtension: fileExtension,
            subdirectory: subdirectory
        ) {
            return url
        }

        // SwiftPM bakes an absolute build-machine path into Bundle.module.
        // A distributed .app must be self-contained and must never evaluate
        // that fallback after it has been moved to another Mac.
        guard primaryBundle.bundleURL.pathExtension.lowercased() != "app" else {
            return nil
        }

        return packageBundle()?.url(
            forResource: name,
            withExtension: fileExtension,
            subdirectory: subdirectory
        )
    }
}
