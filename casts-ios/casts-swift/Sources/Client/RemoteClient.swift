// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation
import Apollo
import casts_c

public final class RemoteClient {
    public static let shared = RemoteClient()
    public private(set) lazy var apollo = ApolloClient(
        url: URL(string: "http://127.0.0.1:3000/graphql")!
    )
}
