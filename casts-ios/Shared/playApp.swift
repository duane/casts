//
//  playApp.swift
//  Shared
//
//  Created by Duane Bailey on 10/24/21.
//

import SwiftUI
import Apollo

@main
struct playApp: App {
    @NSApplicationDelegateAdaptor private var delegate: PlayAppDelegate
    
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

class PlayAppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    let client = ApolloClient(url: URL(string: "http://localhost:3000/graphql")!)
}
