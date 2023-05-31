// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

class LocalDownloadedFile {
    let namespace: String
    let key: String
    let url: URL
    
    private var __path: String?
    private var __downloadDestination: URL?
    private var __data: Data?
    
    init(namespace: String, key: String, url: URL) {
        self.namespace = namespace
        self.key = key
        self.url = url
    }
    
    // file path
    var destinationUrl: URL {
        get {
            if __downloadDestination == nil {
                let urls = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask);
                let url = urls[0];
                __downloadDestination = url.appendingPathComponent(key)
            }
            return __downloadDestination!
        }
    }
    
    var path: String {
        get {
            return destinationUrl.path
        }
    };
    
    var data: Data? {
        get {
            if __data == nil {
                if FileManager.default.fileExists(atPath: self.destinationUrl.path) {
                    if let data = try? Data(contentsOf: self.destinationUrl) {
                        __data = data
                        return data
                    }
                    print("Failed to fetch data from \(self.path), trying to download again")
                }
                if let data = try? Data(contentsOf: self.url) {
                    try? data.write(to: self.destinationUrl)
                    __data = data
                    return data
                }
                return nil
            }
            return __data
        }
    };
}

class LocalDownloadedImage: LocalDownloadedFile {
    #if os(iOS)
    var __image: UIImage? = nil;
    var image: UIImage {
        get {
            if __image == nil {
                if let data = self.data {
                    __image = UIImage(data:data)!
                    return __image!
                }
            }
            return __image!
        }
    }
    #endif

    #if os(macOS)
    var __image: NSImage? = nil;
    var image: NSImage {
        get {
            if __image == nil {
                if let data = self.data {
                    __image = NSImage(data:data)!
                    return __image!
                }
            }
            return __image!
        }
    }
    #endif
}

class FeedLogo: LocalDownloadedImage {
    init(feedId: UUID, url: URL) {
        super.init(namespace: "logos", key: feedId.uuidString, url: url)
    }
}

final class FeedLogoManager {
    static let shared: FeedLogoManager = FeedLogoManager()
    private var cache: NSCache = NSCache<NSURL, FeedLogo>()
    init() {
        self.cache = NSCache<NSURL, FeedLogo>()
        self.cache.name = "Logo Cache"
    }
    
    #if os(iOS)
    func image(feedId: UUID, url: URL) -> UIImage {
        if let feedLogo = self.cache.object(forKey: url as NSURL) {
            return feedLogo.image
        }
        let feedLogo = FeedLogo(feedId: feedId, url: url)
        self.cache.setObject(feedLogo, forKey: url as NSURL)
        return feedLogo.image
    }
    #endif
    
    #if os(macOS)
    func image(feedId: UUID, url: URL) -> NSImage {
        if let feedLogo = self.cache.object(forKey: url as NSURL) {
            return feedLogo.image
        }
        let feedLogo = FeedLogo(feedId: feedId, url: url)
        self.cache.setObject(feedLogo, forKey: url as NSURL)
        return feedLogo.image
    }
    #endif
}
