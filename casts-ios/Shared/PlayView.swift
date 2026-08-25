// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import AVFoundation

struct PlayView: View {
    let feed: GrpcInterface_Feed
    let entry: GrpcInterface_Entry
    
    private let formatter = DateFormatter()
    
    @State private var playing = false
    @State private var mediaError: String? = nil
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
                SquareImageView(
                    feedId: UUID(uuidString: self.feed.feedID)!,
                    url: URL(string: self.feed.logo.url)!
                ).frame(
                    minWidth: 100,
                    maxWidth: 500,
                    minHeight: 100,
                    maxHeight: 500
                )
            } else {
                Image(systemName: "mic.circle").imageScale(.large).font(.system(size: 100))
            }
            Text(self.feed.title)
            if let author = self.feed.authors.first {
                Text(author)
            }
            Text(formatter.string(from: Date.now))
            if let message = mediaError ?? state.playbackError {
                Text(message).font(.callout).foregroundColor(.red)
            }

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
                    if !self.state.isPlaying {
                        self.reportPlayback()
                    }
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
            let audioContent = entry.media
                .flatMap { $0.content }
                .first { $0.contentType.hasPrefix("audio/") }
            guard let audioContent, let url = URL(string: audioContent.url) else {
                self.mediaError = "This entry has no playable audio."
                return
            }
            self.state.prepareToPlay(url: url)
            // Resume from the last reported playback position.
            if entry.elapsedSeconds > 0 {
                let position = Float64(entry.elapsedSeconds)
                self.state.seek(position)
                self.state.currentTime = position
            }
        }.onDisappear {
            if self.state.isPlaying {
                self.state.togglePlayState()
            }
            self.reportPlayback()
        }
    }

    /// Report the position reached to the server; last write wins.
    private func reportPlayback() {
        let seconds = self.state.currentTime
        guard seconds.isFinite, seconds > 0 else {
            return
        }
        var request = GrpcInterface_RecordPlaybackRequest()
        request.streamID = FeedListViewModel.streamID
        request.feedID = self.feed.feedID
        request.entryGuid = self.entry.guid
        request.elapsedSeconds = Int64(seconds)
        request.elapsedNanos = Int32((seconds - seconds.rounded(.down)) * 1_000_000_000)
        _ = AppDelegate.client.recordPlayback(request)
    }
}
