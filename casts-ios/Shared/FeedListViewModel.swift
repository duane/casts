//
//  FeedListViewModel.swift
//  play
//
//  Created by Duane Bailey on 5/6/23.
//

import Foundation
import Generated

final class FeedListViewModel: ObservableObject {
    func fetch() {
        let query = FetchFeedsQuery()
        GraphQLClient.shared.fetch(query: query)
    }
}
