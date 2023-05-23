// Copyright (c) Duane R. Bailey 2023. All rights reserved.

import SwiftUI

struct SquareImageView: View {
    @State var image: UIImage? = nil
    var url: URL

    var body: some View {
        VStack {
            if image != nil {
                Image(uiImage: image!).resizable().aspectRatio(contentMode: .fill)
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
        SquareImageView(url: URL(string: "https://placehold.co/600x400.png")!)
    }
}
