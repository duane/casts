// @generated
// This file was automatically generated and should not be edited.

@_exported import ApolloAPI

public class FetchFeedsQuery: GraphQLQuery {
  public static let operationName: String = "FetchFeeds"
  public static let document: ApolloAPI.DocumentType = .notPersisted(
    definition: .init(
      #"""
      query FetchFeeds($streamId: Uuid!) {
        feeds(streamId: $streamId) {
          __typename
          feedId
          title
          description
          entries {
            __typename
            guid
            title
            summary
            content {
              __typename
              body
              contentType
              length
              src {
                __typename
                href
                hrefLang
                rel
              }
            }
          }
        }
      }
      """#
    ))

  public var streamId: Uuid

  public init(streamId: Uuid) {
    self.streamId = streamId
  }

  public var __variables: Variables? { ["streamId": streamId] }

  public struct Data: Casts.SelectionSet {
    public let __data: DataDict
    public init(_dataDict: DataDict) { __data = _dataDict }

    public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Query }
    public static var __selections: [ApolloAPI.Selection] { [
      .field("feeds", [Feed].self, arguments: ["streamId": .variable("streamId")]),
    ] }

    public var feeds: [Feed] { __data["feeds"] }

    /// Feed
    ///
    /// Parent Type: `Feed`
    public struct Feed: Casts.SelectionSet {
      public let __data: DataDict
      public init(_dataDict: DataDict) { __data = _dataDict }

      public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Feed }
      public static var __selections: [ApolloAPI.Selection] { [
        .field("__typename", String.self),
        .field("feedId", Casts.Uuid.self),
        .field("title", String.self),
        .field("description", String.self),
        .field("entries", [Entry].self),
      ] }

      public var feedId: Casts.Uuid { __data["feedId"] }
      public var title: String { __data["title"] }
      public var description: String { __data["description"] }
      public var entries: [Entry] { __data["entries"] }

      /// Feed.Entry
      ///
      /// Parent Type: `Entry`
      public struct Entry: Casts.SelectionSet {
        public let __data: DataDict
        public init(_dataDict: DataDict) { __data = _dataDict }

        public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Entry }
        public static var __selections: [ApolloAPI.Selection] { [
          .field("__typename", String.self),
          .field("guid", String.self),
          .field("title", String.self),
          .field("summary", String.self),
          .field("content", Content?.self),
        ] }

        public var guid: String { __data["guid"] }
        public var title: String { __data["title"] }
        public var summary: String { __data["summary"] }
        public var content: Content? { __data["content"] }

        /// Feed.Entry.Content
        ///
        /// Parent Type: `Content`
        public struct Content: Casts.SelectionSet {
          public let __data: DataDict
          public init(_dataDict: DataDict) { __data = _dataDict }

          public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Content }
          public static var __selections: [ApolloAPI.Selection] { [
            .field("__typename", String.self),
            .field("body", String?.self),
            .field("contentType", String.self),
            .field("length", Int?.self),
            .field("src", Src?.self),
          ] }

          public var body: String? { __data["body"] }
          public var contentType: String { __data["contentType"] }
          public var length: Int? { __data["length"] }
          public var src: Src? { __data["src"] }

          /// Feed.Entry.Content.Src
          ///
          /// Parent Type: `Link`
          public struct Src: Casts.SelectionSet {
            public let __data: DataDict
            public init(_dataDict: DataDict) { __data = _dataDict }

            public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Link }
            public static var __selections: [ApolloAPI.Selection] { [
              .field("__typename", String.self),
              .field("href", String.self),
              .field("hrefLang", String?.self),
              .field("rel", String?.self),
            ] }

            public var href: String { __data["href"] }
            public var hrefLang: String? { __data["hrefLang"] }
            public var rel: String? { __data["rel"] }
          }
        }
      }
    }
  }
}
