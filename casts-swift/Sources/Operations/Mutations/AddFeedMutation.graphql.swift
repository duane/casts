// @generated
// This file was automatically generated and should not be edited.

@_exported import ApolloAPI

public class AddFeedMutation: GraphQLMutation {
  public static let operationName: String = "AddFeed"
  public static let document: ApolloAPI.DocumentType = .notPersisted(
    definition: .init(
      #"""
      mutation AddFeed($streamId: Uuid!, $feedId: Uuid!, $url: Url!) {
        addFeed(streamId: $streamId, feedId: $feedId, url: $url)
      }
      """#
    ))

  public var streamId: Uuid
  public var feedId: Uuid
  public var url: Url

  public init(
    streamId: Uuid,
    feedId: Uuid,
    url: Url
  ) {
    self.streamId = streamId
    self.feedId = feedId
    self.url = url
  }

  public var __variables: Variables? { [
    "streamId": streamId,
    "feedId": feedId,
    "url": url
  ] }

  public struct Data: Casts.SelectionSet {
    public let __data: DataDict
    public init(_dataDict: DataDict) { __data = _dataDict }

    public static var __parentType: ApolloAPI.ParentType { Casts.Objects.Mutation }
    public static var __selections: [ApolloAPI.Selection] { [
      .field("addFeed", Casts.Uuid.self, arguments: [
        "streamId": .variable("streamId"),
        "feedId": .variable("feedId"),
        "url": .variable("url")
      ]),
    ] }

    public var addFeed: Casts.Uuid { __data["addFeed"] }
  }
}
