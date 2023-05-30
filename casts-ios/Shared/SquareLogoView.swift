// Copyright (c) Duane R. Bailey 2023. All rights reserved.

import SwiftUI

struct SquareImageView: View {
    #if os(iOS)
    @State var image: UIImage? = nil
    #endif
    
    #if os(macOS)
    @State var image: NSImage? = nil
    #endif

    var feedId: UUID
    var url: URL

    var body: some View {
        VStack {
            if image != nil {
                #if os(iOS)
                Image(uiImage: FeedLogoManager.shared.image(feedId: feedId, url: url)).resizable().aspectRatio(contentMode: .fill)
                #endif
                #if os(macOS)
                Image(nsImage: FeedLogoManager.shared.image(feedId: feedId, url: url)).resizable().aspectRatio(contentMode: .fit)
                #endif
            } else {
                Spacer()
            }
        }.onAppear {
            ImageLoader.loadImage(withUrl: self.url) { image in
                self.image = image
            }
        }
    }
}

struct SquareImageView_Previews: PreviewProvider {
    static var previews: some View {
        SquareImageView(feedId: UUID(), url: URL(string: "https://placehold.co/600x400.png")!)
    }
}
