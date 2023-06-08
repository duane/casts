// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation
import Casts

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [Casts.FetchFeedsQuery.Data.Feed] = []
    
    func fetch() {
        let query = Casts.FetchFeedsQuery(streamId: "34c3c08d-0d55-4af4-a126-170c1380f07e")
        EmbeddedClient.shared.apollo.fetch(query:query) { result in
            do {
                let data = try result.get().data
                DispatchQueue.main.async {
                    self.feeds = data?.feeds ?? []
                }
            } catch {
                fatalError("could not call into gql server: \(error)")
            }
        }
    }
    
    init() {
        fetch()
    }
}
