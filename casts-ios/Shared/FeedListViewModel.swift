// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [GrpcInterface_Feed] = []
    @Published var errorMessage: String? = nil

    static let streamID = playApp.STREAM_ID.uuidString.lowercased()

    func fetch() {
        let client = AppDelegate.client!
        var request = GrpcInterface_ListFeedsRequest()
        request.streamID = Self.streamID
        let start = DispatchTime.now()
        CastsLog.rpc.notice("ListFeeds: start")
        // 20s deadline: without one, a call on a connection that looks
        // `.ready` but has silently died (post-sleep, network switch) waits
        // forever with no feedback -- indistinguishable, to the user, from
        // the app having hung. See CastsConnection.callOptions.
        _ = client.listFeeds(request, callOptions: CastsConnection.callOptions(timeout: .seconds(20))).response.always { result in
            let elapsedMs = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
            DispatchQueue.main.async {
                switch result {
                case let .success(feeds):
                    CastsLog.rpc.notice("ListFeeds: ok, \(feeds.feeds.count) feeds, elapsedMs=\(elapsedMs, privacy: .public)")
                    self.feeds = feeds.feeds
                    self.errorMessage = nil
                case let .failure(err):
                    CastsLog.rpc.error("ListFeeds: failed after elapsedMs=\(elapsedMs, privacy: .public): \(String(describing: err), privacy: .public)")
                    self.errorMessage = "Could not reach the casts server: \(err)"
                }
            }
        }
    }

    func addFeed(urlString: String) {
        let client = AppDelegate.client!
        var request = GrpcInterface_AddFeedRequest()
        request.streamID = Self.streamID
        request.url = urlString
        let start = DispatchTime.now()
        CastsLog.rpc.notice("AddFeed: start")
        // Longer deadline than a plain read: the server fetches and parses
        // the feed live before committing, which can be slow against a slow
        // podcast host.
        _ = client.addFeed(request, callOptions: CastsConnection.callOptions(timeout: .seconds(45))).response.always { result in
            let elapsedMs = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
            DispatchQueue.main.async {
                switch result {
                case .success:
                    CastsLog.rpc.notice("AddFeed: ok, elapsedMs=\(elapsedMs, privacy: .public)")
                    self.errorMessage = nil
                    self.fetch()
                case let .failure(err):
                    CastsLog.rpc.error("AddFeed: failed after elapsedMs=\(elapsedMs, privacy: .public): \(String(describing: err), privacy: .public)")
                    self.errorMessage = "Could not add feed: \(err)"
                }
            }
        }
    }

    init() {
        fetch()
    }
}
