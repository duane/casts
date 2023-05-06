// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation
import Apollo

final class GraphQLClient {
    static let shared = GraphQLClient()
    private(set) lazy var apollo = ApolloClient(
        url: URL(string: "http://127.0.0.1:3000/graphql")!
    )
}
