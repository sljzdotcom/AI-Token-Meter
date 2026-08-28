// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AI-Meter",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "AIMeterCore", targets: ["AIMeterCore"]),
        .executable(name: "AIMeterApp", targets: ["AIMeterApp"]),
    ],
    targets: [
        .target(name: "AIMeterCore"),
        .executableTarget(
            name: "AIMeterApp",
            dependencies: ["AIMeterCore"],
            exclude: ["Resources"]
        ),
        .testTarget(
            name: "AIMeterCoreTests",
            dependencies: ["AIMeterCore"],
            resources: [.process("Fixtures")]
        ),
    ]
)
