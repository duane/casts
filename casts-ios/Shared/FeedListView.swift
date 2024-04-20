// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

struct FeedListView: View {
    @StateObject var feedsModel: FeedListViewModel = FeedListViewModel()
    @Environment(\.openWindow) private var openWindow
    
    var body: some View {
        NavigationStack {
            VStack(alignment: .center, spacing: 0) {
                List(feedsModel.feeds, id: \.feedID) { feed in
                    NavigationLink(destination: FeedDetailView(feed: feed)) {
                        HStack(alignment: .center, spacing: 0) {
                            SquareImageView(feedId: UUID(uuidString: feed.feedID)!, url: URL(string: feed.logo.url)!)
                                    .aspectRatio(contentMode: .fit)
                                    .frame(maxWidth: 50)
                            Text(feed.title).frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }.buttonStyle(.borderless)
                }.listStyle(.plain)
                Button("+") {
                    openWindow(id: "add-feed-by-url-prompt")
                }.font(.system(size: 30)).buttonStyle(.borderless)
            }.navigationTitle("Browse Feeds").padding(0)
        }
    }
}

struct FeedListView_Previews: PreviewProvider {
    static var _feeds: [GrpcInterface_Feed] = []
    static var feeds: [GrpcInterface_Feed] {
        get {
            if _feeds == [] {
                var feed = GrpcInterface_Feed()
                var image = GrpcInterface_Image()
                image.url = "https://placehold.co/400.png"
                
                feed.title = "Example Feed"
                feed.authors = ["Cindy", "Yi"]
                feed.feedID = UUID().uuidString
                feed.description_p = "Some description goes here."
                feed.logo = image

                _feeds.append(feed)
                
                feed = GrpcInterface_Feed()
                image = GrpcInterface_Image()
                image.url = "https://placehold.co/400.png"
                
                feed.title = "Example Feed 2"
                feed.authors = ["Cindy", "Yi"]
                feed.feedID = UUID().uuidString
                feed.description_p = "Some description goes here."
                feed.logo = image

                _feeds.append(feed)
            }
            return _feeds
        }
    }
    
    static var previews: some View {
        FeedListView()
    }
}
