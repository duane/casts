// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation
import Casts

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [Casts.FetchFeedsQuery.Data.Feed] = []
    
    func fetch() {
        let query = Casts.FetchFeedsQuery(streamId: playApp.STREAM_ID.uuidString)
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
