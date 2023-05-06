//
//  GraphQLClient.swift
//  play
//
//  Created by Duane Bailey on 5/6/23.
//

import Foundation
import Apollo

final class GraphQLClient {
    static let shared = GraphQLClient()
    private(set) lazy var apollo = ApolloClient(
        url: URL(string: "http://127.0.0.1:3000/graphql")!
    )
}
