// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import GRPC
import Logging
import CommitStoreSwift

@main
struct commit_store_viewApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    static private(set) var client: GrpcInterface_CommitStoreNIOClient! = nil
    
    func applicationDidFinishLaunching(_ aNotification: Notification) {
        let group = PlatformSupport.makeEventLoopGroup(loopCount: 1)
        var logger = Logger(label: "gRPC", factory: StreamLogHandler.standardOutput(label:))
        logger.logLevel = .info
        let channel = ClientConnection
            .insecure(group: group)
            .withBackgroundActivityLogger(logger)
            .connect(host: "127.0.0.1", port: 5059)
        AppDelegate.client = GrpcInterface_CommitStoreNIOClient(channel: channel)
    }
}
