// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import Casts

struct FeedDetailView: View {
    var feed: FetchFeedsQuery.Data.Feed!
    @State var isShowingDetailView: Bool = false
    
    var body: some View {
        VStack {
            if let logoUrl = feed.logo?.url {
                SquareImageView(url:URL(string:logoUrl)!)
            }
            Text(feed.title)
            Text(feed.description)
            List(feed.entries, id: \.guid) { entry in
                NavigationLink(entry.title, destination: FeedEntryDetailView(entry: entry))
            }.navigationTitle(feed.title)
        }
    }
}

struct FeedDetailView_Previews: PreviewProvider {
    static var previews: some View {
        FeedDetailView()
    }
}
