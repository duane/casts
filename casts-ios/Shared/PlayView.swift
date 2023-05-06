//
//  PlayView.swift
//  Shared
//
//  Created by Duane Bailey on 10/24/21.
//

import SwiftUI
import AVFoundation

struct Metadata {
  let url: URL
  let creator: String
  let title: String
  let author: String
  let published: Date
  let description: String
  let imageURL: URL?
  
  init(url: URL,
       creator: String,
       title: String,
       author: String,
       published: Date,
       description: String,
       imageURL: URL?) {
    self.url = url
    self.creator = creator
    self.title = title
    self.author = author
    self.published = published
    self.description = description
    self.imageURL = imageURL
  }
}

struct PlayView: View {
  private let metadata: Metadata
  private let formatter = DateFormatter()

  @State private var playing = false
  @ObservedObject private var state: PlayerState
  
  init() {
    let url = Bundle.main.url(forResource: "sample", withExtension: "mp3")!
    let imageURL = Bundle.main.url(forResource: "sample", withExtension: "jpg")!

    let html = """
<p>It's casual Friday! Sam and Emma host <a href="https://twitter.com/daveweigel">Dave Weigel</a>, national political correspondent for the Washington Post, as they walk through how the Democrats are approaching the midterms (and what the voters think). Dave, Emma, and Sam jump into how the infrastructure and reconciliation failures are being read by voters, discussing how candidates are bringing it up on the campaign trail, why the average voter sees legislative hold up as evidence of policy failure, and why, despite their inability to emphasize its importance, Democrats should see Republican critiques failing to stick as a success. They also cover Biden’s falling approval rating, and the perspective of people who’d really hoped he would bring back at least some sense of normalcy coming out of COVID. Next, they jump into the Republican culture war shifting the battlegrounds into the schools and the spotlight onto the children, with their aim taking on critical race theory, trans rights, and school-based vax and mask mandates, and look at the VA McAuliffe v. Youngkin race as a key example of how Dems can, or cannot, handle this fight. This brings them to discuss how Democrats are utilizing Trump in the run-up to 2022 midterms, the importance of fear in generating midterm turnout, how they’re capitalizing on 1/6, and they wrap up the interview with a summary of the different tactics Republicans and Democrats are taking into next year. Emma and Sam also discuss the state of reconciliation heading into next week and Biden’s performance on the CNN town hall last night with Anderson Cooper.</p>
    <p>And in the second half Sam and Emma are joined by <a href="https://twitter.com/lafinabsolute">Jason Myles</a>, co-host of the <a href="https://www.thisisrevolutionpodcast.com/">This Is Revolution</a> podcast, as they dissect his Medium article, <a href="https://jasonmyles.medium.com/i-was-a-teenage-anarchist-e918b00bb13">“I Was a Teenage Anarchist,"</a> on the growth of the punk movement around the concepts of authenticity and deconstruction, how that hindered its growth and ability to alter the mainstream, instead, setting it up to be absorbed by the machine, and what that can teach us about the state of the online left today. They also touch on the marketing of anti-capitalism, from Parasite to the Joker, and what it means to be a “sellout.” Emma and Sam wrap up the fun half with Rachel Hamm’s claim to her prophetical witch hunt, Jim Jordan’s objection to protocol, plus, your IMs!</p>
    <p>Purchase tickets for the live show in Boston on January 16th <a href="https://thewilbur.com/artist/majority-report/">HERE!</a></p>
    <p>Become a member at <a href="http://www.jointhemajorityreport.com/">JoinTheMajorityReport.com</a></p>
    <p>Subscribe to the AMQuickie newsletter <a href="https://madmimi.com/signups/170390/join">here.</a></p>
    <p>Join the Majority Report Discord!</p>
    <p><a href="http://majoritydiscord.com/">http://majoritydiscord.com/</a></p>
    <p>Get all your MR merch at our store <a href="https://shop.majorityreportradio.com/">https://shop.majorityreportradio.com/</a></p>
    <p>(Merch issues and concerns can be addressed here: <a href="mailto:majorityreportstore@mirrorimage.com">majorityreportstore@mirrorimage.com</a>)</p>
    <p>You can now watch the livestream on <a href="https://www.twitch.tv/themajorityreport">Twitch</a></p>
    <p>Check out today’s sponsor:</p>
    <p><strong>sunsetlakecbd</strong> is a majority employee owned farm in Vermont, producing 100% pesticide free CBD products. Great company, great product and fans of the show! Use code Leftisbest and get 20% off at <a href="http://www.sunsetlakecbd.com/">http://www.sunsetlakecbd.com</a>. And now Sunset Lake CBD has donated $2500 to the Nurses strike fund, and we encourage MR listeners to help if they can. Here’s a link to where folks can donate: <a href="https://forms.massnurses.org/we-stand-with-st-vincents-nurses/">https://forms.massnurses.org/we-stand-with-st-vincents-nurses/</a></p>
    <p><a href="https://mysolarnerd.hubspotpagebuilder.com/the-brightest-choice-in-solar"><strong>MySolarNerd.com</strong></a><strong>:</strong> There are a lot of homeowners that aren't aware of the solar options currently available. It is now possible to retrofit a home with solar panels for no money down. Most homeowners that switch over to solar see significant savings starting in their first year. This is possible thanks to the Solar Investor Tax Credit (going away soon). My Solar Nerd’s mission is SIMPLE: Help you find the best solar program for your home and make the transition as EASY and SMOOTH as possible. Go to <a href="https://mysolarnerd.hubspotpagebuilder.com/the-brightest-choice-in-solar">mysolarnerd.com</a> and fill out the inquiry form now. Make sure you select Majority Report Listener for how you heard about My Solar Nerd to receive a $200 gift card upon installation!</p>
    <p>Support the St. Vincent Nurses today as they continue to strike for a fair contract! <a href="https://action.massnurses.org/we-stand-with-st-vincents-nurses/">https://action.massnurses.org/we-stand-with-st-vincents-nurses/</a></p>
    <p>Subscribe to Discourse Blog, a newsletter and website for progressive essays and related fun partly run by AM Quickie writer Jack Crosbie. <a href="https://discourseblog.com/">https://discourseblog.com/</a></p>
    <p>Subscribe to AM Quickie writer Corey Pein’s podcast News from Nowhere, at <a href="https://www.patreon.com/newsfromnowhere">https://www.patreon.com/newsfromnowhere</a></p>
    <p>Check out Matt’s show, <a href="https://twitter.com/LeftReckoning?ref_src=twsrc%5Egoogle%7Ctwcamp%5Eserp%7Ctwgr%5Eauthor">Left Reckoning</a>, on <a href="https://www.youtube.com/channel/UCerkndrD3Z6rceekc4emVpA">Youtube</a>, and subscribe on <a href="https://www.patreon.com/leftreckoning">Patreon</a>!</p>
    <p>Subscribe to Matt's other show <a href="https://twitter.com/lithangover?lang=en">Literary Hangover</a> on <a href="https://www.patreon.com/literaryhangover">Patreon</a>!</p>
    <p>Check out The Letterhack’s upcoming Kickstarter project for his new graphic novel! <a href="https://www.kickstarter.com/projects/milagrocomic/milagro-heroe-de-las-calles">https://www.kickstarter.com/projects/milagrocomic/milagro-heroe-de-las-calles</a></p>
    <p>Check out Matt Binder’s <a href="https://www.youtube.com/mattbinder">YouTube</a> channel!</p>
    <p>Subscribe to Brandon's show <a href="https://twitter.com/th3discourse?lang=en">The Discourse</a> on <a href="https://www.patreon.com/ExpandTheDiscourse">Patreon</a>!</p>
    <p>Check out The Nomiki Show live at 3 pm ET on <a href="https://www.youtube.com/user/KonstNomiki">YouTube</a> at <a href="https://www.patreon.com/thenomikishow">patreon.com/thenomikishow</a></p>
    <p>Check out Jamie’s podcast, The Antifada, at <a href="https://www.patreon.com/theantifada">patreon.com/theantifada</a>, on <a href="https://podcasts.apple.com/us/podcast/the-antifada/id1372879721">iTunes</a>, or at <a href="http://twitch.tv/theantifada">twitch.tv/theantifada</a> (streaming every Monday, Wednesday, Thursday and Friday at 7pm ET!)</p>
    <p>Follow the Majority Report crew on Twitter:</p>
    <p>@SamSeder</p>
    <p>@EmmaVigeland</p>
    <p>@MattBinder</p>
    <p>@MattLech</p>
    <p>@BF1nn</p>
    <p>@BradKAlsop</p>
"""

    self.init(metadata:
                Metadata(
                  url: url,
                  creator: "The Majority Report",
                  title: "10/22 What’s on the Ballot & the Criminal Justice Elections That Could Reform Policing w/ Daniel Nichanian",
                  author: "Sam Seder",
                  published: Date(),
                  description: html,
                  imageURL: imageURL))
  }
  
  init(metadata: Metadata) {
    self.formatter.dateStyle = .long
    self.formatter.timeStyle = .none
    self.metadata = metadata
    self.state = PlayerState()
    self.state.prepareToPlay(url: self.metadata.url)
  }
  
  var body: some View {
    VStack {
      Text(self.metadata.creator)
      if self.metadata.imageURL != nil {
          #if os(iOS)
        Image(uiImage: UIImage(contentsOfFile: Bundle.main.path(forResource: "sample", ofType: "jpg")!)!).resizable().scaledToFit()
          #endif
      } else {
        Image(systemName: "mic.circle").imageScale(.large).font(.system(size: 100))
      }
      Text(self.metadata.title)
      Text(self.metadata.author)
      Text(formatter.string(from: self.metadata.published))
      
      HTMLView(content: self.metadata.description)
      
      Spacer()
      HStack {
        Text(state.readableCurrentTime())
        Slider(
          value: $state.currentTime,
          in: 0...state.totalTime,
          onEditingChanged: { editing in
            self.state.seekRequested()
          }
        )
        Text(state.readableTotalTime())
      }
      HStack {
        Spacer()
        Button(action: {
          print("rw")
          self.state.seekDelta(-30)
        }, label: {
          Image(systemName: "gobackward.30").imageScale(.large).font(.system(size: 40))
        })
        Spacer()
        Button(action: {
          self.state.togglePlayState()
        }, label: {
          Image(systemName: self.state.isPlaying ? "pause.circle" : "play.circle").imageScale(.large).font(.system(size: 40))
        })
        Spacer()
        Button(action: {
          print("ff")
          self.state.seekDelta(30)
        }, label: {
          Image(systemName: "goforward.30").imageScale(.large).font(.system(size: 40))
        })
        Spacer()
      }
      HStack {
        Button(state.speed.display()) {
          state.speed = state.speed.next_in_rotation()
        }.font(.system(size: 20)).padding(8)
        Image(systemName: "speaker")
        Slider(
          value: $state.volume,
          in: 0...1
        )
        Image(systemName: "speaker.wave.3")
        RoutePicker().frame(maxWidth: 40, maxHeight: 40)
      }
    }
  }
}

struct PlayView_Previews: PreviewProvider {
  static var previews: some View {
    PlayView()
  }
}
