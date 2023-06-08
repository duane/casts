// swift-tools-version:5.7

import PackageDescription

let package = Package(
  name: "Casts",
  platforms: [
    .iOS(.v12),
    .macOS(.v10_14),
    .tvOS(.v12),
    .watchOS(.v5),
  ],
  products: [
    .library(name: "Casts", targets: ["Casts"]),
  ],
  dependencies: [
    .package(url: "https://github.com/apollographql/apollo-ios.git", from: "1.0.0"),
  ],
  targets: [
    .target(
      name: "Casts",
      dependencies: [
        .product(name: "ApolloAPI", package: "apollo-ios"),
        "casts_c"
      ],
      path: "./Sources",
      linkerSettings: [
        .linkedLibrary("casts_c"),
        .unsafeFlags(["-L../target/debug"])
      ]
    ),
    .systemLibrary(name: "casts_c"),
    .testTarget(name: "casts-Tests", dependencies: ["Casts"])
  ]
)
