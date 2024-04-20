// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

final class ImageLoader {
    static var session: URLSession {
        let configuration = URLSessionConfiguration.default
        configuration.waitsForConnectivity = true
        configuration.timeoutIntervalForRequest = 60 // seconds that a task will wait for data to arrive
        configuration.timeoutIntervalForResource = 300 // seconds for whole resource request to complete ,.
        return URLSession(configuration: configuration)
    }
    
    static func loadImage(url: URL) async -> Data? {
        let result = (try? await URLSession.shared.data(from: url))
        if result == nil {
            return nil
        }
        let (data, response) = result!
        guard let httpResponse = response as? HTTPURLResponse,
                  httpResponse.statusCode == 200 else {
            assert(false);
            }
        return data;
    }
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
