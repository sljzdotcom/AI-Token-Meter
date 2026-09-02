// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AI-Meter",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "AIMeterCore", targets: ["AIMeterCore"]),
        .executable(name: "AIMeterApp", targets: ["AIMeterApp"]),
        .executable(name: "AIMeterWidgetExtension", targets: ["AIMeterWidgetExtension"]),
    ],
    targets: [
        .binaryTarget(
            name: "Sparkle",
            url: "https://github.com/sparkle-project/Sparkle/releases/download/2.9.4/Sparkle-for-Swift-Package-Manager.zip",
            checksum: "cb6fdbdc8884f15d62a616e79face92b08322410fd2d425edc6596ccbf4ba3b0"
        ),
        .target(name: "AIMeterCore"),
        .executableTarget(
            name: "AIMeterApp",
            dependencies: [
                "AIMeterCore",
                "Sparkle",
            ],
            exclude: [
                "Resources/Info.plist",
                "Resources/AIMeterApp.entitlements",
            ],
            resources: [
                .copy("Resources/Logos"),
                .copy("Resources/Backgrounds"),
            ]
        ),
        .executableTarget(
            name: "AIMeterWidgetExtension",
            dependencies: ["AIMeterCore"],
            exclude: [
                "Resources/Info.plist",
                "Resources/AITokenMeterWidget.entitlements",
            ],
            resources: [
                .copy("Resources/Logos"),
                .copy("Resources/Backgrounds"),
            ],
            swiftSettings: [.unsafeFlags(["-application-extension"])]
        ),
        .testTarget(
            name: "AIMeterCoreTests",
            dependencies: ["AIMeterCore"],
            resources: [.process("Fixtures")]
        ),
        .testTarget(
            name: "AIMeterAppTests",
            dependencies: ["AIMeterApp"]
        ),
        .testTarget(
            name: "AIMeterWidgetExtensionTests",
            dependencies: ["AIMeterWidgetExtension"]
        ),
    ]
)
