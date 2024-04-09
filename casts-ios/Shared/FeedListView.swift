// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

struct FeedListView: View {
    @StateObject var feedsModel: FeedListViewModel = FeedListViewModel()
    @Environment(\.openWindow) private var openWindow
    
    var body: some View {
        NavigationStack {
            VStack {
                List(feedsModel.feeds, id: \.feedID) { feed in
                    NavigationLink(feed.title, destination: FeedDetailView(feed: feed))
                }
                Button("+") {
                    openWindow(id: "add-feed-by-url-prompt")
                }.font(.system(size: 30)).buttonStyle(.borderless)
            }.navigationTitle("Browse Feeds")
        }
    }
}

struct FeedListView_Previews: PreviewProvider {
    static var previews: some View {
        FeedListView()
    }
}
