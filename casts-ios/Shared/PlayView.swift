// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import AVFoundation
import Casts

struct PlayView: View {
    let feed: FetchFeedsQuery.Data.Feed!
    let entry: FetchFeedsQuery.Data.Feed.Entry!
    
    private let formatter = DateFormatter()
    
    @State private var playing = false
    @ObservedObject private var state: PlayerState
    
    init(feed: FetchFeedsQuery.Data.Feed,
         entry: FetchFeedsQuery.Data.Feed.Entry) {
        self.formatter.dateStyle = .long
        self.formatter.timeStyle = .none
        self.feed = feed
        self.entry = entry
        self.state = PlayerState()
    }
    
    var body: some View {
        VStack {
//            Text(self.feed.authors[0])
            if self.feed.logo?.url != nil {
#if os(iOS)
                Image(uiImage: UIImage(contentsOfFile: Bundle.main.path(forResource: "sample", ofType: "jpg")!)!).resizable().scaledToFit()
#endif
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
            let mediaObjects: [FetchFeedsQuery.Data.Feed.Entry.Medium] = entry.media;
            let audioObject: FetchFeedsQuery.Data.Feed.Entry.Medium? = mediaObjects.first(where: {$0.content[0].contentType == "audio/mpeg"})
            let url = URL(string: (audioObject?.content[0].url)!)!
            self.state.prepareToPlay(url: url)
        }
    }
}
