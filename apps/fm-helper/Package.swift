// swift-tools-version:5.9
// The on-device model helper: asks the model built into macOS, through
// Apple's Foundation Models framework, what someone in a World would say,
// with a read-only tool over the facts the World gave it. See main.swift.
import PackageDescription

let package = Package(
    name: "fm-helper",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(name: "fm-helper", path: "Sources/fm-helper")
    ]
)
