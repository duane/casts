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
            ContentView()
        }
    }
    #endif
    
    #if os(macOS)
    var body: some Scene {
        Window("Casts", id: "main-window") {
            ContentView()
        }
        .windowToolbarStyle(.unifiedCompact)
        .windowResizability(.contentSize)
        .windowStyle(.hiddenTitleBar)
    }
    #endif
}

#if os(iOS)
class AppDelegate: NSObject, UIApplicationDelegate, ObservableObject {
    static private(set) var _client: GrpcInterface_CastsNIOClient? = nil
    
    static var client: GrpcInterface_CastsNIOClient! {
        get {
            if self._client == nil {
                let group = PlatformSupport.makeEventLoopGroup(loopCount: 1)
                var logger = Logger(label: "gRPC", factory: StreamLogHandler.standardOutput(label:))
                logger.logLevel = .info
                do {
                    self._client = try CastsConnection.makeClient(group: group, logger: logger)
                } catch {
                    // Fail loudly: without a client the app can do nothing,
                    // and a silent nil here would surface much later as an
                    // unexplained empty library.
                    fatalError("could not connect to casts: \(error)")
                }
            }
            return self._client!
        }
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
        do {
            AppDelegate.client = try CastsConnection.makeClient(group: group, logger: logger)
        } catch {
            fatalError("could not connect to casts: \(error)")
        }
    }
}
#endif
