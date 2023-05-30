// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation
import Casts

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [FetchFeedsQuery.Data.Feed] = []
    
    func fetch() {
        let query = FetchFeedsQuery(streamId: "34c3c08d-0d55-4af4-a126-170c1380f07e")
        GraphQLClient.shared.apollo.fetch(query:query) { result in
            guard let data = try! result.get().data else { fatalError("could not call into gql server") }
            DispatchQueue.main.async {
                self.feeds = data.feeds
            }
        }
    }
    
    init() {
        fetch()
    }
}
