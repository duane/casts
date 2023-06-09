// Copyright (c) 2023 by Duane R Bailey. All rights reserved.

import SwiftUI
import Casts

func isUrl(val: String) -> Bool {
    let url = URL(string: val)
    return url?.scheme != nil
    && url?.host != nil
    && url?.path != nil
}

struct AddFeedWindow: View {
    @Environment(\.dismiss) private var dismiss
    @State var text: String = ""
    @State var isValid: Bool? = nil
    
    var body: some View {
        VStack {
            Text("Look up a feed by URL:")
            TextField("URL to look up", text: $text)
                .lineLimit(3, reservesSpace: true)
                .multilineTextAlignment(.leading)
                .onChange(of:text) { value in
                    self.isValid = text.isEmpty ? nil : isUrl(val: text)
                }
            Button("Submit") {
                let newUuid = UUID()
                let mutation = Casts.AddFeedMutation(streamId: playApp.STREAM_ID.uuidString, feedId: newUuid.uuidString, url: self.text)
                let cancellable = EmbeddedClient.shared.apollo.perform(mutation: mutation)
                print("Added feed")
                self.text = ""
                self.isValid = nil
                dismiss()
            }.disabled(isValid != true)
        }
    }
}

struct AddFeedWindow_Previews: PreviewProvider {
    static var previews: some View {
        AddFeedWindow()
    }
}
