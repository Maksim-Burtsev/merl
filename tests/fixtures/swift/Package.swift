// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Shop",
    targets: [
        .target(name: "Warehouse"),
        .target(name: "Core", dependencies: ["Warehouse"], path: "Sources/swift-core"),
        .target(name: "Shop", dependencies: ["Warehouse", "Core"]),
        .executableTarget(name: "Basins"),
        .testTarget(name: "ShopTests", dependencies: ["Shop"]),
    ]
)
