//
//  FeedEntryDetailView.swift
//  play
//
//  Created by Duane Bailey on 5/6/23.
//

import SwiftUI
import Casts

struct FeedEntryDetailView: View {
    var entry: FetchFeedsQuery.Data.Feed.Entry!
    
    var body: some View {
        Text(entry.title).navigationTitle(entry.title)
    }
}

//struct FeedEntryDetailView_Previews: PreviewProvider {
//    static var previews: some View {
//    }
//}


