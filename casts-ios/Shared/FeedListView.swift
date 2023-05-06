// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

struct FeedListView: View {
    @StateObject var feedsModel: FeedListViewModel = FeedListViewModel()
    var body: some View {
        NavigationStack {
            List(feedsModel.feeds, id: \.feedId) { feed in
                NavigationLink(feed.title, destination: FeedDetailView(feed: feed))
            }.navigationTitle("Browse Feeds")
        }
    }
}

struct FeedListView_Previews: PreviewProvider {
    static var previews: some View {
        FeedListView()
    }
}
