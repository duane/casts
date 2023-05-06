// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import WebKit
import SwiftUI

#if os(iOS)
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
#endif

#if os(macOS)
struct HTMLView: NSViewRepresentable {
    typealias NSViewType = WKWebView
    let content: String
    
    init(content: String) {
        self.content = content
    }
     
    func makeNSView(context: Context) -> WKWebView {
        return WKWebView()
    }
    
    func updateNSView(_ uiView: WKWebView, context: Context) {
        uiView.loadHTMLString(content, baseURL: nil)
    }
}
#endif
