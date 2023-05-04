public struct CommitStoreSwift {

    public init() {}
}

extension GrpcInterface_Commit: Identifiable {
    public var id: String { commitID }
}
