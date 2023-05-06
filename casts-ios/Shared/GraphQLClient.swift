//
//  GraphQLClient.swift
//  play
//
//  Created by Duane Bailey on 5/6/23.
//

import Foundation
import Apollo

final class GraphQLClient {
    static let shared = Network()
    private(set) lazy var apollo = ApolloClient(
        url: URL(string: "http://localhost:3000")!
    )
}
