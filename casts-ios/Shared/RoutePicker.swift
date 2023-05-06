//
//  RoutePicker.swift
//  play
//
//  Created by Duane Bailey on 5/2/23.
//


#if os(iOS)
import UIKit
#endif
import SwiftUI
import AVKit

#if os(iOS)
struct RoutePicker: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
      return AVRoutePickerView()
    }

    func updateUIView(_ uiView: UIView, context: Context) {}
}
#endif

#if os(macOS)
struct RoutePicker: NSViewRepresentable {
    func makeNSView(context: Context) -> AVRoutePickerView {
        return AVRoutePickerView()
    }
    
    func updateNSView(_ nsView: AVRoutePickerView, context: Context) {}
    
    typealias NSViewType = AVRoutePickerView
}
#endif
