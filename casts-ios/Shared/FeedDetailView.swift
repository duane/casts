// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI


struct FeedDetailView: View {
    var feed: GrpcInterface_Feed
    @State var isShowingDetailView: Bool = false

    var body: some View {
        VStack {
            if feed.hasLogo {
                SquareImageView(feedId: UUID(uuidString: feed.feedID)!, url:URL(string:feed.logo.url)!)
            }
            Text(feed.title)
            Text(feed.description_p)
            List(feed.entries, id: \.guid) { entry in
                NavigationLink(entry.title, destination: PlayView(feed: feed, entry: entry))
            }.navigationTitle(feed.title)
        }
    }
}
//
//struct FeedDetailView_Previews: PreviewProvider {
//    static var previews: some View {
////        FeedDetailView()
//    }
//}
