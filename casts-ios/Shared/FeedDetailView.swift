// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI


struct FeedDetailView: View {
    var feed: GrpcInterface_Feed
    @State var isShowingDetailView: Bool = false

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                if feed.hasLogo {
                    SquareImageView(feedId: UUID(uuidString: feed.feedID)!, url: URL(string: feed.logo.url)!)
                        .frame(maxWidth: 200, maxHeight: 200)
                }
                Text(feed.title)
                Text(feed.description_p)
                ForEach(feed.entries, id: \.guid) { entry in
                    NavigationLink { PlayView(feed: feed, entry: entry) } label: {
                        FeedEntryRow(entry: entry)
                    }.buttonStyle(.borderless)
                }.navigationTitle(feed.title)
            }
        }
    }
}
//
struct FeedDetailView_Previews: PreviewProvider {
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
                
                var entries: [GrpcInterface_Entry] = []
                var entry = GrpcInterface_Entry()
                entry.title = "Ep 01: Introducing our nefarious hosts"
                entry.guid = UUID().uuidString
                entries.append(entry)
                
                entry = GrpcInterface_Entry()
                entry.title = "Ep 02: Expanding Branden's morbid past"
                entry.guid = UUID().uuidString
                entries.append(entry)
                
                entry = GrpcInterface_Entry()
                entry.title = "Ep 03: Learning about Lee's mother and his love life"
                entry.guid = UUID().uuidString
                entries.append(entry)
                
                entry = GrpcInterface_Entry()
                entry.title = "Ep 04: We learn about how Lee met Branden"
                entry.guid = UUID().uuidString
                entries.append(entry)
                
                entry = GrpcInterface_Entry()
                entry.title = "Ep 05: Sara stops by to regale us with a drinking tale about Fly Away Home"
                entry.guid = UUID().uuidString
                entries.append(entry)
                
                entry = GrpcInterface_Entry()
                entry.title = "Ep 06: Dee manages to slip one past Branden as they explored their checkered board game past"
                entry.guid = UUID().uuidString
                entries.append(entry)
                
                feed.entries = entries
                _feeds = [feed]
            }
            return _feeds
        }
    }
    
    static var previews: some View {
        FeedDetailView(feed: self.feeds[0])
    }
}
