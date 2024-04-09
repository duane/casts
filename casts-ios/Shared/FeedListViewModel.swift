// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [DummyFeed] = []
    
    func fetch() {
//        let query = Casts.FetchFeedsQuery(streamId: playApp.STREAM_ID.uuidString)
//        let subscription = Casts.WatchFeedsSubscription(streamId: playApp.STREAM_ID.uuidString)
//        let subscribed = EmbeddedClient.shared.apollo.subscribe(subscription: subscription) { result in
//            do {
//                let data = try result.get().data
//                DispatchQueue.main.async {
//                    self.feeds = data?.watchFeeds ?? []
//                }
//            } catch {
//                fatalError("could not call into gql server: \(error)")
//            }
//        }
    }
    
    init() {
        fetch()
    }
}
