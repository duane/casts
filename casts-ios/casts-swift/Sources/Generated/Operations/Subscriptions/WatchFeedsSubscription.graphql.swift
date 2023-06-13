// @generated
// This file was automatically generated and should not be edited.

@_exported import ApolloAPI

public extension Casts {
  class WatchFeedsSubscription: GraphQLSubscription {
    public static let operationName: String = "WatchFeeds"
    public static let document: ApolloAPI.DocumentType = .notPersisted(
      definition: .init(
        #"""
        subscription WatchFeeds($streamId: Uuid!) {
          watchFeeds(streamId: $streamId) {
            __typename
            ...FeedMetadata
            entries {
              __typename
              ...EntryMetadata
            }
          }
        }
        """#,
        fragments: [FeedMetadata.self, EntryMetadata.self]
      ))

    public var streamId: Uuid

    public init(streamId: Uuid) {
      self.streamId = streamId
    }

    public var __variables: Variables? { ["streamId": streamId] }

    public struct Data: Casts.SelectionSet {
      public let __data: DataDict
      public init(_dataDict: DataDict) { __data = _dataDict }

      public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Subscription }
      public static var __selections: [ApolloAPI.Selection] { [
        .field("watchFeeds", [WatchFeed].self, arguments: ["streamId": .variable("streamId")]),
      ] }

      public var watchFeeds: [WatchFeed] { __data["watchFeeds"] }

      /// WatchFeed
      ///
      /// Parent Type: `Feed`
      public struct WatchFeed: Casts.SelectionSet {
        public let __data: DataDict
        public init(_dataDict: DataDict) { __data = _dataDict }

        public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Feed }
        public static var __selections: [ApolloAPI.Selection] { [
          .field("__typename", String.self),
          .field("entries", [Entry].self),
          .fragment(FeedMetadata.self),
        ] }

        /// A list of entries to which the feed links.
        public var entries: [Entry] { __data["entries"] }
        /// The ID of the feed.
        public var feedId: Casts.Uuid { __data["feedId"] }
        /// The title of the feed.
        public var title: String { __data["title"] }
        /// Authors of the feed
        public var authors: [String] { __data["authors"] }
        /// A string describing the feed.
        public var description: String { __data["description"] }
        /// A link to an image with which to represent the feed.
        public var logo: FeedMetadata.Logo? { __data["logo"] }

        public struct Fragments: FragmentContainer {
          public let __data: DataDict
          public init(_dataDict: DataDict) { __data = _dataDict }

          public var feedMetadata: FeedMetadata { _toFragment() }
        }

        /// WatchFeed.Entry
        ///
        /// Parent Type: `Entry`
        public struct Entry: Casts.SelectionSet {
          public let __data: DataDict
          public init(_dataDict: DataDict) { __data = _dataDict }

          public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Entry }
          public static var __selections: [ApolloAPI.Selection] { [
            .field("__typename", String.self),
            .fragment(EntryMetadata.self),
          ] }

          public var guid: String { __data["guid"] }
          public var title: String { __data["title"] }
          public var summary: String { __data["summary"] }
          public var media: [EntryMetadata.Medium] { __data["media"] }
          public var content: EntryMetadata.Content? { __data["content"] }

          public struct Fragments: FragmentContainer {
            public let __data: DataDict
            public init(_dataDict: DataDict) { __data = _dataDict }

            public var entryMetadata: EntryMetadata { _toFragment() }
          }
        }
      }
    }
  }

}