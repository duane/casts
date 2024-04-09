// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [GrpcInterface_Feed] = []
    
    func fetch() {
        let client = AppDelegate.client!
        var request = GrpcInterface_ListFeedsRequest();
        request.streamID="34c3c08d-0d55-4af4-a126-170c1380f07e";
        _ = client.listFeeds(request).response.always { result in
            switch result {
            case let .success(feeds):
                let data = feeds
                DispatchQueue.main.async {
                    self.feeds = data.feeds
                }
            case let .failure(err):
                fatalError("could not call into casts server: \(err)")
            }
        }
    }
    
    init() {
        fetch()
    }
}
