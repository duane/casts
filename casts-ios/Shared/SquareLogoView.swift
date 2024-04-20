// Copyright (c) Duane R. Bailey 2023. All rights reserved.

import SwiftUI

struct SquareImageView: View {
    #if os(iOS)
    @State var image: UIImage? = nil
    #endif
    
    #if os(macOS)
    @State var image: NSImage? = nil
    #endif
    
    @State var appeared: Bool = false

    var feedId: UUID
    var url: URL

    var body: some View {
        VStack {
            if self.image == nil {
                Image(systemName: "antenna.radiowaves.left.and.right.circle.fill")
                    .resizable()
                    .aspectRatio(contentMode: .fit)
            } else {
                #if os(iOS)
                Image(uiImage: self.image!)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                #endif
                #if os(macOS)
                Image(nsImage: self.image!)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                #endif
            }
        }.onAppear {
            Task {
                var components = URLComponents(url: self.url, resolvingAgainstBaseURL: true)
                if components != nil {
                    components!.scheme = "https"
                    let url = components!.url
                    if url != nil {
                        let data = await ImageLoader.loadImage(url: url!);
                        if data != nil {
                            self.image = NSImage(data: data!)
                        }
                    }
                }
            }
        }
    }
}

struct SquareImageView_Previews: PreviewProvider {
    static var previews: some View {
        SquareImageView(feedId: UUID(), url: URL(string: "https://placehold.co/600x400.png")!)
    }
}
