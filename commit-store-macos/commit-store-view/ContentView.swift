// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI
import CommitStoreSwift

struct ContentView: View {
    @State var streamID: String = "E6C1E8A3-06EB-4243-9147-0303E059ABE5"
    @State var fetched: String? = nil
    @State var commits: [GrpcInterface_Commit] = []
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                TextField("stream id", text: $streamID)
                Button("Fetch") {
                    fetched = streamID
                    streamID = ""
                    let cursor = GrpcInterface_Cursor.with {
                        $0.streamID = fetched!
                        $0.maxVersion = Int64.max
                    }
                    let result = AppDelegate.client.streamStreamRange(cursor) { commit in
                        DispatchQueue.main.async {
                            commits.append(commit)
                        }
                    }
                }
            }
            if fetched != nil {
                Text("Fetched stream \(fetched!)")
            }
            Table(commits) {
                TableColumn("Commit ID", value: \.commitID)
                TableColumn("Stream Version") { commit in
                    Text(String(commit.streamVersion))
                }
                TableColumn("Timestamp", value: \.commitTimestamp.date.description)
                TableColumn("Sequence") { commit in
                    Text(String(commit.commitSequence))
                }
                TableColumn("Metadata") { commit in
                    Text(String(data:commit.metadata, encoding: .utf8) ?? "<encoding error>")
                }
                TableColumn("Events") { commit in
                    Text(String(data:commit.events, encoding: .utf8) ?? "<encoding error>")
                }
                TableColumn("Event Count") {
                    Text(String($0.eventsCount))
                }
            }
        }
        .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
