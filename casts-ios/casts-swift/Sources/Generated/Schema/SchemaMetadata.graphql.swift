// @generated
// This file was automatically generated and should not be edited.

import ApolloAPI

public protocol Casts_SelectionSet: ApolloAPI.SelectionSet & ApolloAPI.RootSelectionSet
where Schema == Casts.SchemaMetadata {}

public protocol Casts_InlineFragment: ApolloAPI.SelectionSet & ApolloAPI.InlineFragment
where Schema == Casts.SchemaMetadata {}

public protocol Casts_MutableSelectionSet: ApolloAPI.MutableRootSelectionSet
where Schema == Casts.SchemaMetadata {}

public protocol Casts_MutableInlineFragment: ApolloAPI.MutableSelectionSet & ApolloAPI.InlineFragment
where Schema == Casts.SchemaMetadata {}

public extension Casts {
  typealias ID = String

  typealias SelectionSet = Casts_SelectionSet

  typealias InlineFragment = Casts_InlineFragment

  typealias MutableSelectionSet = Casts_MutableSelectionSet

  typealias MutableInlineFragment = Casts_MutableInlineFragment

  enum SchemaMetadata: ApolloAPI.SchemaMetadata {
    public static let configuration: ApolloAPI.SchemaConfiguration.Type = SchemaConfiguration.self

    public static func objectType(forTypename typename: String) -> Object? {
      switch typename {
      case "Mutation": return Casts.Objects.Mutation
      case "Subscription": return Casts.Objects.Subscription
      case "Feed": return Casts.Objects.Feed
      case "Image": return Casts.Objects.Image
      case "Link": return Casts.Objects.Link
      case "Query": return Casts.Objects.Query
      case "Entry": return Casts.Objects.Entry
      case "MediaObject": return Casts.Objects.MediaObject
      case "MediaContent": return Casts.Objects.MediaContent
      case "Content": return Casts.Objects.Content
      default: return nil
      }
    }
  }

  enum Objects {}
  enum Interfaces {}
  enum Unions {}

}