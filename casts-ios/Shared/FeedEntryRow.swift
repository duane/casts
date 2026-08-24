//
//  FeedEntryRow.swift
//  play
//
//  Created by Duane Bailey on 4/9/24.
//

import SwiftUI

struct FeedEntryRow: View {
    @State var entry: GrpcInterface_Entry
    
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 0) {
                Text(casualTimeAgo(date: entry.published.date))
                Text(entry.title)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                if let media = entry.media.first {
                    Text(casualDuration(elapsed: TimeInterval(media.durationSecs)))
                }
            }.padding(EdgeInsets(top: 0, leading: 10, bottom: 0, trailing: 10))
            Divider()
                .frame(height: 1)
                .background(Color(white: 0.8))
        }
        
    }
}

#Preview {
    var entry = GrpcInterface_Entry()
    entry.title = "Ep 01: Introducing our nefarious hosts"
    entry.guid = UUID().uuidString
    entry.published = toTimestamp(date: Date() - 10000)
    
    var media = GrpcInterface_MediaObject()
    media.durationSecs = 5280;
    media.durationNsecs = 0;
    
    entry.media.append(media)
    return FeedEntryRow(entry: entry)
}
