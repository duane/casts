// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import Foundation
import AVFoundation
import Combine

enum Speed {
  case oh_five, one, one_five, two
  
  func display() -> String {
    switch self {
    case .oh_five:
      return "0.5x"
    case .one:
      return "1x"
    case .one_five:
      return "1.5x"
    case .two:
      return "2x"
    }
  }
  
  func next_in_rotation() -> Speed {
    switch self {
    case .oh_five:
      return .one
    case .one:
      return .one_five
    case .one_five:
      return .two
    case .two:
      return .oh_five
    }
  }
  
  func rate() -> Float {
    switch self {
    case .oh_five:
      return 0.5
    case .one:
      return 1
    case .one_five:
      return 1.5
    case .two:
      return 2
    }
  }
}

class PlayerState: NSObject, ObservableObject {
  var player: AVPlayer! = nil
  var asset: AVAsset! = nil
  var playerItem: AVPlayerItem! = nil
  var playerItemContext: String! = "foobar"

  @Published var isPlaying = false

  @Published var currentTime = Float64.nan
  @Published var totalTime = Float64.nan
  
  @Published var volume: Float = 1 {
    didSet {
      self.player.volume = volume
    }
  }
  
  @Published var speed: Speed = .one {
    didSet {
      self.player.rate = speed.rate()
    }
  }
  
  private var timer: Timer? = nil
  
  func readableCurrentTime() -> String {
    readableTime(self.currentTime)
  }
  
  func readableTotalTime() -> String {
    readableTime(self.totalTime)
  }
  
  func readableTime(_ seconds: Float64) -> String {
    if seconds.isNaN {
      return "-:-:-"
    } else {
      let totalSeconds = Int(ceil(seconds))
      let hours = totalSeconds / 3600
      let minutes = (totalSeconds % 3600) / 60
      let seconds = (totalSeconds % 3600) % 60
      return String(format: "%02i:%02i:%02i", hours, minutes, seconds)
    }
  }
  
  func togglePlayState() {
    if self.isPlaying {
      self.isPlaying = false
      self.player.pause()
      self.timer = nil
    } else {
      self.isPlaying = true
      self.player.play()
      self.timer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { (t) in
        self.currentTime = CMTimeGetSeconds(self.playerItem.currentTime())
     }
    }
  }
  
  func seekDelta(_ delta: Float64) {
    var to = self.currentTime + delta;
    if to < 0 {
      to = 0
    }
    if to > self.totalTime {
      to = self.totalTime
    }
    self.seek(to)
  }
  
  func seekRequested() {
    self.seek(self.currentTime)
  }
  
  func seek(_ to: Float64) {
    let ctime = CMTime(seconds: to, preferredTimescale: 1)
    self.player.seek(to: ctime, toleranceBefore: CMTime.zero, toleranceAfter: CMTime.zero)
  }
  
  func prepareToPlay(url: URL) {
    self.asset = AVAsset(url: url)
    self.totalTime = CMTimeGetSeconds(self.asset.duration)
    
    self.playerItem = AVPlayerItem(asset: asset,
                                   automaticallyLoadedAssetKeys: ["playable"])
    playerItem.addObserver(self,
                           forKeyPath: #keyPath(AVPlayerItem.status),
                           options: [.old, .new],
                           context: &self.playerItemContext)
    self.player = AVPlayer(playerItem: playerItem)
    self.player.seek(to: CMTime(seconds: 0, preferredTimescale: 1))
    self.player.volume = 0
    self.volume = self.player.volume
    
    self.currentTime = CMTimeGetSeconds(self.playerItem.currentTime())
  }
  
  override func observeValue(forKeyPath keyPath: String?,
                             of object: Any?,
                             change: [NSKeyValueChangeKey : Any]?,
                             context: UnsafeMutableRawPointer?) {
    // Only handle observations for the playerItemContext
    guard context == &self.playerItemContext else {
        super.observeValue(forKeyPath: keyPath,
                           of: object,
                           change: change,
                           context: context)
        return
    }
    
    if keyPath == #keyPath(AVPlayerItem.status) {
      let status: AVPlayerItem.Status
      
      // Get the status change from the change dictionary
      if let statusNumber = change?[.newKey] as? NSNumber {
        status = AVPlayerItem.Status(rawValue: statusNumber.intValue)!
      } else {
        status = .unknown
      }
      
      // Switch over the status
      switch status {
      case .readyToPlay:
        break
      case .failed:
        break
      case .unknown:
        break
      default:
        break
      }
    }
  }
}
