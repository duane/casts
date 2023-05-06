//
//  InMemoryTransport.swift
//  play
//
//  Created by Duane Bailey on 5/6/23.
//

import Foundation
import Apollo
import ApolloAPI

enum InMemoryTransportError: Error {
    case runtimeError(String)
}

class InMemoryTransport: NetworkTransport {
    
    
    func send<Operation>(operation: Operation, cachePolicy: Apollo.CachePolicy, contextIdentifier: UUID?, callbackQueue: DispatchQueue, completionHandler: @escaping (Result<Apollo.GraphQLResult<Operation.Data>, Error>) -> Void) -> Apollo.Cancellable where Operation : ApolloAPI.GraphQLOperation {
        fatalError("not implemented")
    }
}
