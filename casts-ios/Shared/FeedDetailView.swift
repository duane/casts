// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

import SwiftUI

/// Pages one feed's entries out of `ListEntries`.
///
/// A feed's entries used to arrive embedded in the `Feed` that `ListFeeds`
/// returned, so this view had them already. That does not survive a real
/// library: the embedded form made every response unbounded in the size of
/// the feed, and `ListFeeds` reached 78 MB -- past what grpc-swift accepts,
/// so the list showed nothing at all. Entries now come from a paging
/// endpoint, one bounded page per round trip.
///
/// Lives beside the view rather than in its own file so that adding it costs
/// no `project.pbxproj` edit across both targets.
final class FeedEntriesViewModel: ObservableObject {
    @Published var entries: [GrpcInterface_Entry] = []
    @Published var errorMessage: String? = nil
    @Published var isLoading = false

    /// Empty once the last page has arrived. Distinct from "no entries yet":
    /// the server sends a token only when more actually remain, so this is
    /// never a guess from a full-looking page.
    private var nextPageToken: String = ""
    private var reachedEnd = false
    private var feedID: String = ""

    /// Entries per round trip. Below the server's own cap, so the server
    /// never has to clamp; small enough that the first screenful appears
    /// without waiting for a long back-catalogue.
    private static let pageSize: Int32 = 50

    init() {}

    /// Seed a preview or a test with fixed entries and no network.
    init(previewEntries: [GrpcInterface_Entry]) {
        entries = previewEntries
        reachedEnd = true
    }

    /// Load the first page for `feedID`, if it has not been loaded already.
    ///
    /// Idempotent because SwiftUI may run `.task` again for the same view;
    /// without the guard, revisiting a feed would append a second copy of
    /// every entry.
    func start(feedID: String) {
        guard self.feedID != feedID else { return }
        self.feedID = feedID
        entries = []
        nextPageToken = ""
        reachedEnd = false
        errorMessage = nil
        loadNextPage()
    }

    /// Ask for the next page. Safe to call from a row appearing on screen:
    /// it does nothing when a request is already in flight or the last page
    /// has arrived.
    func loadMoreIfNeeded(after entry: GrpcInterface_Entry) {
        guard entry.guid == entries.last?.guid else { return }
        loadNextPage()
    }

    func retry() {
        errorMessage = nil
        loadNextPage()
    }

    private func loadNextPage() {
        guard !isLoading, !reachedEnd, !feedID.isEmpty else { return }
        guard let client = AppDelegate.client else {
            errorMessage = "Not connected to the casts server."
            return
        }
        isLoading = true

        var request = GrpcInterface_ListEntriesRequest()
        request.streamID = FeedListViewModel.streamID
        request.feedID = feedID
        request.limit = Self.pageSize
        request.pageToken = nextPageToken

        let start = DispatchTime.now()
        let pageNumber = entries.count / Int(Self.pageSize) + 1
        CastsLog.rpc.notice("ListEntries: start, page=\(pageNumber)")
        // Same 20s deadline as the other reads: a connection that looks
        // `.ready` but has silently died otherwise waits forever, which the
        // user cannot tell from the app having hung.
        _ = client.listEntries(request, callOptions: CastsConnection.callOptions(timeout: .seconds(20))).response.always { result in
            let elapsedMs = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
            DispatchQueue.main.async {
                self.isLoading = false
                switch result {
                case let .success(page):
                    CastsLog.rpc.notice("ListEntries: ok, \(page.entries.count) entries, more=\(!page.nextPageToken.isEmpty), elapsedMs=\(elapsedMs, privacy: .public)")
                    self.entries.append(contentsOf: page.entries)
                    self.nextPageToken = page.nextPageToken
                    self.reachedEnd = page.nextPageToken.isEmpty
                    self.errorMessage = nil
                case let .failure(err):
                    CastsLog.rpc.error("ListEntries: failed after elapsedMs=\(elapsedMs, privacy: .public): \(String(describing: err), privacy: .public)")
                    self.errorMessage = "Could not load episodes: \(err)"
                }
            }
        }
    }
}

struct FeedDetailView: View {
    var feed: GrpcInterface_Feed
    @State var isShowingDetailView: Bool = false
    @StateObject private var entriesModel = FeedEntriesViewModel()

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                if feed.hasLogo {
                    SquareImageView(feedId: UUID(uuidString: feed.feedID)!, url: URL(string: feed.logo.url)!)
                        .frame(maxWidth: 200, maxHeight: 200)
                }
                Text(feed.title)
                Text(feed.description_p)
                if let message = entriesModel.errorMessage {
                    VStack(spacing: 8) {
                        Text(message)
                            .font(.callout)
                            .multilineTextAlignment(.center)
                        Button("Retry") { entriesModel.retry() }
                    }.padding(10)
                }
                ForEach(entriesModel.entries, id: \.guid) { entry in
                    NavigationLink { PlayView(feed: feed, entry: entry) } label: {
                        FeedEntryRow(entry: entry)
                    }
                    .buttonStyle(.borderless)
                    // Paging is driven by the last row reaching the screen,
                    // so a feed with thousands of episodes costs one round
                    // trip per screenful instead of one huge response.
                    .onAppear { entriesModel.loadMoreIfNeeded(after: entry) }
                }.navigationTitle(feed.title)
                if entriesModel.isLoading {
                    ProgressView().padding(10)
                }
            }
        }
        .task { entriesModel.start(feedID: feed.feedID) }
    }
}

/// The preview seeds entries directly: previews have no server to page from.
struct FeedDetailView_Previews: PreviewProvider {
    static var feed: GrpcInterface_Feed {
        var feed = GrpcInterface_Feed()
        var image = GrpcInterface_Image()
        image.url = "https://placehold.co/400.png"

        feed.title = "Example Feed"
        feed.authors = ["Cindy", "Yi"]
        feed.feedID = UUID().uuidString
        feed.description_p = "Some description goes here."
        feed.logo = image
        return feed
    }

    static var previews: some View {
        FeedDetailView(feed: feed)
    }
}
