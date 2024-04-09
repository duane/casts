// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import AVFoundation

struct PlayView: View {
    let feed: GrpcInterface_Feed
    let entry: GrpcInterface_Entry
    
    private let formatter = DateFormatter()
    
    @State private var playing = false
    @ObservedObject private var state: PlayerState
    
    init(feed: GrpcInterface_Feed,
         entry: GrpcInterface_Entry) {
        self.formatter.dateStyle = .long
        self.formatter.timeStyle = .none
        self.feed = feed
        self.entry = entry
        self.state = PlayerState()
    }
    
    var body: some View {
        VStack {
            if self.feed.hasLogo && self.feed.logo.url != "" {
                SquareImageView(feedId: UUID(uuidString: self.feed.feedID)!, url: URL(string: self.feed.logo.url)!)
            } else {
                Image(systemName: "mic.circle").imageScale(.large).font(.system(size: 100))
            }
            Text(self.feed.title)
            Text(self.feed.authors[0])
            Text(formatter.string(from: Date.now))
            
            HTMLView(content: self.entry.summary)
            
            Spacer()
            HStack {
                Text(state.readableCurrentTime())
                Slider(
                    value: $state.currentTime,
                    in: 0...(state.totalTime.isNaN ? 0 : state.totalTime),
                    onEditingChanged: { editing in
                        self.state.seekRequested()
                    }
                )
                Text(state.readableTotalTime())
            }
            HStack {
                Spacer()
                Button(action: {
                    self.state.seekDelta(-30)
                }, label: {
                    Image(systemName: "gobackward.30").imageScale(.large).font(.system(size: 40))
                }).buttonStyle(.borderless)
                Spacer()
                Button(action: {
                    self.state.togglePlayState()
                }) {
                    Image(systemName: self.state.isPlaying ? "pause.circle" : "play.circle").imageScale(.large).font(.system(size: 40))
                }.buttonStyle(.borderless)
                Spacer()
                Button(action: {
                    self.state.seekDelta(30)
                }, label: {
                    Image(systemName: "goforward.30").imageScale(.large).font(.system(size: 40))
                }).buttonStyle(.borderless)
                Spacer()
            }
            HStack {
                Button(state.speed.display()) {
                    state.speed = state.speed.next_in_rotation()
                }.font(.system(size: 20)).padding(8).buttonStyle(.borderless)
                Image(systemName: "speaker")
                Slider(
                    value: $state.volume,
                    in: 0...1
                )
                Image(systemName: "speaker.wave.3")
                RoutePicker().frame(maxWidth: 40, maxHeight: 40)
            }
        }.onAppear {
            let mediaObjects: [GrpcInterface_MediaObject] = entry.media;
            let audioObject: GrpcInterface_MediaObject? = mediaObjects.first(where: {$0.content[0].contentType == "audio/mpeg"})
            let url = URL(string: (audioObject?.content[0].url)!)!
            self.state.prepareToPlay(url: url)
        }
    }
}
