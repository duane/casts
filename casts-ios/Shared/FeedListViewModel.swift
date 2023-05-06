//
//  FeedListViewModel.swift
//  play
//
//  Created by Duane Bailey on 5/6/23.
//

import Foundation
import Casts

final class FeedListViewModel: ObservableObject {
    @Published var feeds: [FetchFeedsQuery.Data.Feed] = []
    
    func fetch() {
        let query = FetchFeedsQuery(streamId: "E6C1E8A3-06EB-4243-9147-0303E059ABE5")
        GraphQLClient.shared.apollo.fetch(query:query) { result in
            guard let data = try! result.get().data else { fatalError("could not call into gql server") }
            print("got \(data.feeds.count) entries")
            DispatchQueue.main.async {
                self.feeds = data.feeds
            }
        }
    }
    
    init() {
        fetch()
    }
}
