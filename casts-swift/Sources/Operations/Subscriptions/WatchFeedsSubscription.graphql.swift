// @generated
// This file was automatically generated and should not be edited.

@_exported import ApolloAPI

public class WatchFeedsSubscription: GraphQLSubscription {
  public static let operationName: String = "WatchFeeds"
  public static let document: ApolloAPI.DocumentType = .notPersisted(
    definition: .init(
      #"""
      subscription WatchFeeds($streamId: Uuid!) {
        watchFeeds(streamId: $streamId) {
          __typename
          feedId
          title
          authors
          description
          logo {
            __typename
            url
            rawUrl
            title
            link {
              __typename
              href
              rel
              mediaType
              hrefLang
              title
              length
            }
            width
            height
            description
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
        .field("feedId", Casts.Uuid.self),
        .field("title", String.self),
        .field("authors", [String].self),
        .field("description", String.self),
        .field("logo", Logo?.self),
      ] }

      /// The ID of the feed.
      public var feedId: Casts.Uuid { __data["feedId"] }
      /// The title of the feed.
      public var title: String { __data["title"] }
      /// Authors of the feed
      public var authors: [String] { __data["authors"] }
      /// A string describing the feed.
      public var description: String { __data["description"] }
      /// A link to an image with which to represent the feed.
      public var logo: Logo? { __data["logo"] }

      /// WatchFeed.Logo
      ///
      /// Parent Type: `Image`
      public struct Logo: Casts.SelectionSet {
        public let __data: DataDict
        public init(_dataDict: DataDict) { __data = _dataDict }

        public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Image }
        public static var __selections: [ApolloAPI.Selection] { [
          .field("__typename", String.self),
          .field("url", Casts.Url?.self),
          .field("rawUrl", String.self),
          .field("title", String?.self),
          .field("link", Link?.self),
          .field("width", Int?.self),
          .field("height", Int?.self),
          .field("description", String?.self),
        ] }

        public var url: Casts.Url? { __data["url"] }
        public var rawUrl: String { __data["rawUrl"] }
        public var title: String? { __data["title"] }
        public var link: Link? { __data["link"] }
        public var width: Int? { __data["width"] }
        public var height: Int? { __data["height"] }
        public var description: String? { __data["description"] }

        /// WatchFeed.Logo.Link
        ///
        /// Parent Type: `Link`
        public struct Link: Casts.SelectionSet {
          public let __data: DataDict
          public init(_dataDict: DataDict) { __data = _dataDict }

          public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Link }
          public static var __selections: [ApolloAPI.Selection] { [
            .field("__typename", String.self),
            .field("href", String.self),
            .field("rel", String?.self),
            .field("mediaType", String?.self),
            .field("hrefLang", String?.self),
            .field("title", String?.self),
            .field("length", Int?.self),
          ] }

          public var href: String { __data["href"] }
          public var rel: String? { __data["rel"] }
          public var mediaType: String? { __data["mediaType"] }
          public var hrefLang: String? { __data["hrefLang"] }
          public var title: String? { __data["title"] }
          public var length: Int? { __data["length"] }
        }
      }
    }
  }
}
