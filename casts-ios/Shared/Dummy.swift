//
//  Dummy.swift
//  play
//
//  Created by Duane Bailey on 4/8/24.
//

import Foundation

struct DummyMediaContent {
    var url: String
    var contentType: String
}

struct DummyMediaObject {
    var content: [DummyMediaContent]
}

struct DummyLogo {
    var url: String?
}

struct DummyEntry {
    var guid: String
    var url: String
    var title: String
    var summary: String
    var media: [DummyMediaObject]
}

struct DummyFeedMetadata {
    
}

struct DummyFeed {
    var feedId: String
    var logo: DummyLogo?
    var title: String
    var description: String
    var authors: [String]
    var metadata: DummyFeedMetadata?
    var entries: [DummyEntry]
}
