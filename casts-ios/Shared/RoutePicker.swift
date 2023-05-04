//
//  RoutePicker.swift
//  play
//
//  Created by Duane Bailey on 10/24/21.
//

import UIKit
import SwiftUI
import AVKit

struct RoutePicker: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
      return AVRoutePickerView()
    }

    func updateUIView(_ uiView: UIView, context: Context) {}
}
