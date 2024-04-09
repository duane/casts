// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#if os(iOS)
import UIKit
#endif
import SwiftUI
import AVKit

#if os(iOS)
struct RoutePicker: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
        let view = AVRoutePickerView();
//        view.isRoutePickerButtonBordered = false;
        return view;
    }

    func updateUIView(_ uiView: UIView, context: Context) {}
}
#endif

#if os(macOS)
struct RoutePicker: NSViewRepresentable {
    func makeNSView(context: Context) -> AVRoutePickerView {
        let view = AVRoutePickerView();
        view.isRoutePickerButtonBordered = false;
        return view;
    }
    
    func updateNSView(_ nsView: AVRoutePickerView, context: Context) {}
    
    typealias NSViewType = AVRoutePickerView
}
#endif
