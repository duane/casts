// @generated
// This file was automatically generated and should not be edited.

@_exported import ApolloAPI

public extension Casts {
  struct EntryMetadata: Casts.SelectionSet, Fragment {
    public static var fragmentDefinition: StaticString { """
      fragment EntryMetadata on Entry {
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
      """ }

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

    /// Medium
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

      /// Medium.Content
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

    /// Content
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

      /// Content.Src
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