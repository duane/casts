// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import Logging
import GRPC

@main
struct playApp: App {
#if os(iOS)
    @UIApplicationDelegateAdaptor private var appDelegate: AppDelegate
#endif
#if os(macOS)
    @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate
#endif
    static let STREAM_ID: UUID = UUID(uuidString: "34c3c08d-0d55-4af4-a126-170c1380f07e")!
    
    #if os(iOS)
    var body: some Scene {
        WindowGroup {
            Text("Hello, world")
        }
    }
    #endif
    
    #if os(macOS)
    var body: some Scene {
        Window("Casts", id: "main-window") {
            ContentView()
        }
//        Window("Add feed", id: "add-feed-by-url-prompt") {
//            AddFeedWindow().fixedSize(horizontal: false, vertical: true)
//        }
        .windowToolbarStyle(.unifiedCompact)
        .windowResizability(.contentSize)
        .windowStyle(.hiddenTitleBar)
    }
    #endif
}

#if os(iOS)
class AppDelegate: NSObject, UIApplicationDelegate, ObservableObject {
    static private(set) var client: GrpcInterface_CastsNIOClient! = nil
    
    func applicationDidFinishLaunching(_ aNotification: Notification) {
        let group = PlatformSupport.makeEventLoopGroup(loopCount: 1)
        var logger = Logger(label: "gRPC", factory: StreamLogHandler.standardOutput(label:))
        logger.logLevel = .info
        let channel = ClientConnection
            .insecure(group: group)
            .withBackgroundActivityLogger(logger)
            .connect(host: "127.0.0.1", port: 5059)
        AppDelegate.client = GrpcInterface_CastsNIOClient(channel: channel)
    }
}
#endif

#if os(macOS)
class AppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    static private(set) var client: GrpcInterface_CastsNIOClient! = nil
    
    func applicationWillFinishLaunching(_ aNotification: Notification) {
        let group = PlatformSupport.makeEventLoopGroup(loopCount: 1)
        var logger = Logger(label: "gRPC", factory: StreamLogHandler.standardOutput(label:))
        logger.logLevel = .info
        let channel = ClientConnection
            .insecure(group: group)
            .withBackgroundActivityLogger(logger)
            .connect(host: "127.0.0.1", port: 7392)
        AppDelegate.client = GrpcInterface_CastsNIOClient(channel: channel)
        var request = GrpcInterface_ListFeedsRequest();
        request.streamID="34c3c08d-0d55-4af4-a126-170c1380f07e";
        _ = AppDelegate.client.listFeeds(request).response.always { result in
            print("woah result is \(result)")
            switch result {
            case let .success(feeds):
                let count = feeds.feeds.count
                print("found \(count)")
            case let .failure(err):
                print("err: \(err)")
            }
        }
    }
}
#endif
