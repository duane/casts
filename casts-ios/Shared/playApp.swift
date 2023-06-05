// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import Apollo

@main
struct playApp: App {
    var body: some Scene {
        Window("Casts", id: "main-window") {
            ContentView()
        }
        Window("Add feed", id: "add-feed-by-url-prompt") {
            AddFeedWindow().fixedSize(horizontal: false, vertical: true)
        }
        .windowToolbarStyle(.unifiedCompact)
        .windowResizability(.contentSize)
        .windowStyle(.hiddenTitleBar)
    }
}


