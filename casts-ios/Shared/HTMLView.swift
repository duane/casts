//
//  HTMLStringView.swift
//  play
//
//  Created by Duane Bailey on 10/24/21.
//

import WebKit
import SwiftUI

struct HTMLView: UIViewRepresentable {
  let content: String
  
  init(content: String) {
    self.content = content
  }
  
  func makeUIView(context: Context) -> WKWebView {
    return WKWebView()
  }
  
  func updateUIView(_ uiView: WKWebView, context: Context) {
    uiView.loadHTMLString(content, baseURL: nil)
  }
}
