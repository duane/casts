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
        _ = client.listFeeds(request).response.always { result in
            DispatchQueue.main.async {
                switch result {
                case let .success(feeds):
                    self.feeds = feeds.feeds
                    self.errorMessage = nil
                case let .failure(err):
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
        _ = client.addFeed(request).response.always { result in
            DispatchQueue.main.async {
                switch result {
                case .success:
                    self.errorMessage = nil
                    self.fetch()
                case let .failure(err):
                    self.errorMessage = "Could not add feed: \(err)"
                }
            }
        }
    }

    init() {
        fetch()
    }
}
