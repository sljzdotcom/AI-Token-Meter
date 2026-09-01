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
        .target(name: "AIMeterCore"),
        .executableTarget(
            name: "AIMeterApp",
            dependencies: ["AIMeterCore"],
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
