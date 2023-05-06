// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

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


