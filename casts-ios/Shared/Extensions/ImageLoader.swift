// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

final class ImageLoader {
#if os(iOS)
    static func loadImage(withUrl url: URL, onComplete: @escaping (UIImage) -> Void) {
        DispatchQueue.global().async {
            if let imageData = try? Data(contentsOf: url) {
                print("maybe got an image: \(imageData.debugDescription)")
                if let image = UIImage(data: imageData) {
                    print("found some image")
                    DispatchQueue.main.async {
                        print("completed!")
                        onComplete(image)
                    }
                }
            }
        }
    }
#endif
    
#if os(macOS)
    static func loadImage(withUrl url: URL, onComplete: @escaping (NSImage) -> Void) {
        DispatchQueue.global().async {
            if let imageData = try? Data(contentsOf: url) {
                print("maybe got an image: \(imageData.debugDescription)")
                if let image = NSImage(data: imageData) {
                    print("found some image")
                    DispatchQueue.main.async {
                        print("completed!")
                        onComplete(image)
                    }
                }
            }
        }
    }
#endif
}
