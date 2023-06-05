// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

final class ImageLoader {
#if os(iOS)
    static func loadImage(withUrl url: URL, onComplete: @escaping (UIImage) -> Void) {
        DispatchQueue.global().async {
            if let imageData = try? Data(contentsOf: url) {
                if let image = UIImage(data: imageData) {
                    DispatchQueue.main.async {
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
                if let image = NSImage(data: imageData) {
                    DispatchQueue.main.async {
                        onComplete(image)
                    }
                }
            }
        }
    }
#endif
}
