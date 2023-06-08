// @generated
// This file was automatically generated and should not be edited.

@_exported import ApolloAPI

public extension Casts {
  class FetchFeedsQuery: GraphQLQuery {
    public static let operationName: String = "FetchFeeds"
    public static let document: ApolloAPI.DocumentType = .notPersisted(
      definition: .init(
        #"""
        query FetchFeeds($streamId: Uuid!) {
          feeds(streamId: $streamId) {
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
            entries {
              __typename
              guid
              title
              summary
              media {
                __typename
                content {
                  __typename
                  url
                  contentType
                  height
                  width
                  size
                }
                title
                description
                duration
              }
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
          .field("authors", [String].self),
          .field("description", String.self),
          .field("logo", Logo?.self),
          .field("entries", [Entry].self),
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
        /// A list of entries to which the feed links.
        public var entries: [Entry] { __data["entries"] }

        /// Feed.Logo
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

          /// Feed.Logo.Link
          ///
          /// Parent Type: `Link`
          public struct Link: Casts.SelectionSet {
            public let __data: DataDict
            public init(_dataDict: DataDict) { __data = _dataDict }

            public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Link }
            public static var __selections: [ApolloAPI.Selection] { [
              .field("__typename", String.self),
              .field("href", Casts.Url.self),
              .field("rel", String?.self),
              .field("mediaType", String?.self),
              .field("hrefLang", String?.self),
              .field("title", String?.self),
              .field("length", Int?.self),
            ] }

            public var href: Casts.Url { __data["href"] }
            public var rel: String? { __data["rel"] }
            public var mediaType: String? { __data["mediaType"] }
            public var hrefLang: String? { __data["hrefLang"] }
            public var title: String? { __data["title"] }
            public var length: Int? { __data["length"] }
          }
        }

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
            .field("media", [Medium].self),
            .field("content", Content?.self),
          ] }

          public var guid: String { __data["guid"] }
          public var title: String { __data["title"] }
          public var summary: String { __data["summary"] }
          public var media: [Medium] { __data["media"] }
          public var content: Content? { __data["content"] }

          /// Feed.Entry.Medium
          ///
          /// Parent Type: `MediaObject`
          public struct Medium: Casts.SelectionSet {
            public let __data: DataDict
            public init(_dataDict: DataDict) { __data = _dataDict }

            public static var __parentType: ApolloAPI.ParentType { Casts.Objects.MediaObject }
            public static var __selections: [ApolloAPI.Selection] { [
              .field("__typename", String.self),
              .field("content", [Content].self),
              .field("title", String?.self),
              .field("description", String?.self),
              .field("duration", [Int]?.self),
            ] }

            /// Collection of the media content elements
            public var content: [Content] { __data["content"] }
            /// Title of the object (from the media:title element)
            public var title: String? { __data["title"] }
            /// Short description of the media object (from the media:description element)
            public var description: String? { __data["description"] }
            /// Duration of the object
            public var duration: [Int]? { __data["duration"] }

            /// Feed.Entry.Medium.Content
            ///
            /// Parent Type: `MediaContent`
            public struct Content: Casts.SelectionSet {
              public let __data: DataDict
              public init(_dataDict: DataDict) { __data = _dataDict }

              public static var __parentType: ApolloAPI.ParentType { Casts.Objects.MediaContent }
              public static var __selections: [ApolloAPI.Selection] { [
                .field("__typename", String.self),
                .field("url", Casts.Url?.self),
                .field("contentType", String?.self),
                .field("height", Int?.self),
                .field("width", Int?.self),
                .field("size", Int?.self),
              ] }

              public var url: Casts.Url? { __data["url"] }
              public var contentType: String? { __data["contentType"] }
              public var height: Int? { __data["height"] }
              public var width: Int? { __data["width"] }
              public var size: Int? { __data["size"] }
            }
          }

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
                .field("href", Casts.Url.self),
                .field("hrefLang", String?.self),
                .field("rel", String?.self),
              ] }

              public var href: Casts.Url { __data["href"] }
              public var hrefLang: String? { __data["hrefLang"] }
              public var rel: String? { __data["rel"] }
            }
          }
        }
      }
    }
  }

}