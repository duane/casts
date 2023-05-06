{{TOC}}

# The `casts` podcast manager
`casts` refers to a collection of software libraries and components which can be combined into useful podcast clients and servers. It's intended to be used across multiple clients, devices, and servers to provide seamless experience listening to podcasts without worrying about manually syncing state. It is currently under development and provided publicly here for the interest of people I send the link to.

## The problem
There's a lot of different podcasts players out there and they're all roughly the same: they index a bunch of podcasts, manage automatically downloading and cleaning up associated assets, and if they're really good they'll sync this state across devices. Here is a summary of the problems I have using other podcasting solutions:

1. Changes aren’t synced properly. Feeds added to one device don’t show up in other devices; play state goes missing; episodes I expect to be downloaded aren’t.
2. Changes are only synced across a subset of platforms I use. For instance I find Apple Podcasts to be a suitable enough client, but I sometimes want to listen on devices that can’t easily access iCloud or the state of the podcasts app, and I really don’t want to write that myself. A similar issue exists with many of the existing high-quality podcast management apps that exist as free software.
3. Functionality I want simply doesn’t exist in many clients, or exists as metaphors I find awkward. For instance, I would like to apply rules to my feeds to filter out certain “preview” episodes, or combine multiple different feeds into one, or even to rewrite episode titles to remove things a user finds annoying.
4. Speed/access to native features. Web clients cannot easily control caching of client, and electron interfaces like Spotify stick out like a sore thumb, starts and syncs slowly, and lacks access to native features that I take for granted.
5. It's hard to see a solution to this without a) an agreed upon protocol, maybe something like POP for podcasts or b) writing an identical app for every front-end with another server for syncing changes between them, itself not an easy problem to solve.
## Current offering

`casts` exists as both sets of server and client packages—instead of relying on some server-as-a-service like firebase—because I want semantic processing in the cloud in a straightforward fashion and because I want a higher degree of flexibility and ease of authentication than the solutions I examined offered. The core application logic is written in Rust and is modeled with Martin Fowler’s Event-Driven Architecture in mind as a loose model for how I intend to solve the problem of syncing similar applications across many devices. I have interpreted this to model the application as being broken down into the following components:

- commit-store: a component of software enabling accessing “streams” of “commits”, which are themselves a list of events being committed at the same time. `commit-store-macos` provides a native app that simply streams commits for a given `stream_id` in realtime for monitoring.
- immutable-feed: a rust library to provide a common data type to use that can be converted to and from serialized XML feeds (thanks to `feed-rs`) and the aggregate.
- casts:
	* `FeedAggregate` — this provides the logic to represent the stream of events as an “aggregate”, so the sum of all the events may be evaluated as a single value. This is designed to be serialized to a) provide caching and b) the ability to snapshot to reduce the work necessary to replay an event stream. This type is designed to be immutable, and an event may be applied to an aggregate to produce a newer aggregate with a greater version.
	* `FeedAggregateCommand` — this type offers a set of mutations that, if presented valid values, produce a series of events to commit. This uses the aggregate to validate the parameters and produces at most one commit. The command may be committed with the commit metadata in addition to other details (e.g. user-session data).
	* The `GraphQL` module and associated `serve` binary constitute the server application state. The queries offered here are driven from a hydrated `FeedAggregate`, but might be backed by a SQL server or multiple other projections in the future.
	* `introspect` allows producing the GraphQL schema as a `GraphQLs` schema file.
- `juniper-swift` provides a simple macro that exports a single C symbol and multiple header files that allow binding with `casts` as a static library.
- `casts-swift` is a `swift` package that exposes the GraphQL  schema of `casts` as domain swift queries designed to work with the `Apollo` client.
`casts-ios` is an XCode project that acts as a client consuming the `casts` GraphQL schema.
### Embedded GraphQL? What?
To tackle this problem, GraphQL will make an appearance. This brings four extremely useful things:
1. Building with rust enables binding via a single C symbol that is the entry point into the GraphQL request/response. The rest of the `ABI` is generated from JSON. End-user interface implementations will get "free" serialization of the app model thanks to the prevalence of high-quality GraphQL clients. I didn't include this spike in this mono-repo, but I have already verified how easily SwiftUI code can directly use the streamed queries in the interface code itself. Hopefully, this will enable most swift code to be related to presentation and managing device-specific behavior, like "offline-mode".
2. It has built-in command-query separation. This allows us to event-source the application state. The events themselves are useful primitives for resolving conflicts when syncing state between the client and the server. This is basically the same approach as a CRDT, but I'm aiming for a greatly limited subset of that functionality for most events. Really, only "deleting" a podcast presents a significant problem for conflicts.
3. When online, the embedded GraphQL client can act as both the application state and as a proxy for relaying this state to the shared server, which should ideally have identical application state. Even in proxy mode, the app can provide immediate interaction.
4. The backing store—basically, a consistent series of json blobs—is highly abstracted away. This means that the backing store could be an in-memory sqlite client, a distributed key-value store that has both range scans and a compare-and-swap atomic primitive, or even something like `raft` or `zookeeper`.
5. The bug surface area of the collection of clients across N clients with 1 server is going to look something like: `commit-store bugs + graphql-schema bugs + application bugs + iOS bugs + android bugs + web-app bugs` , i.e it scales linearly with respect to `# of clients`, which is great.
### Benefits of this approach
Use of GraphQL allows the simplification of the application interface to a series of commands and queries. High-quality clients and caches in multiple environments mean extensive boilerplate can be generated automatically in domains convenient to present. For offline use, the GraphQL server can be composed and embedded and communicated with over in-memory RPC, basically allowing the client-server connection to double as an ABI to drive local-only application state, backed by a local store. Synchronization between the client and server is simplified because modeling application state as a series of commits, where commits have a 1:1 relation with end-user actions, where commits are modeled as a series of events, allows fine-grained and highly semantic conflict resolution in a format that is convenient to present to the end-user for resolution. Keeping the communication between the components serializable and conforming to simple protocols should allow a high degree of scalability—the same interface for a dispatcher or a projection might be used on an in-memory atomic queue or a distributed task queue. The backing commit store—which is a highly consistent sequence of events indexed by a stream identifier responsible for dispatching events when commits occur—is generic enough it may be implemented by a number of back-end stores. In the short term I’m targeting sqlite for local usage and postgres for remote development, but it’s also enables usage of any other store which implements joint compare-and-swap and range-scan functionality. By extracting the application logic to working with events, this allows a high degree of flexibility in both syncing writes to the stream across devices and in implementing presentation of the stream to the user in a domain that is most amenable to the end-user experience.
### Alternatives Considered
- iCloud + CloudKit addressed my needs for macOS, but I've had issues with syncing in the past, and it doesn't address my desires for a client on, say, an android phone, or a *nix command-line client.
- Firebase provides decent syncing of arbitrary state, but I wanted a less commercial and more home-brew friendly approach, and this leaves the issue of how to de-duplicate application logic across platforms, especially offline.
- QT and Electron both provide excellent cross-platform UI options, but both of them still require a good deal of extra effort to provide a native experience, and both of them come with sore points (QT is inexorably tied to both C++ and preprocessing, and electron has a high constant factor in terms  of memory use and only taps into native functionality with some difficulty).
- Implementing this in e.g. Lua and just embedding across N front-ends solves a lot, but it still requires a lot of work manually generating interoperability between the application state and the language of presentation.
- Writing custom front-ends in each language means the same application bugs have the chance to be made across every platform—a `platform count * application size` complexity of potential bugs. Not acceptable and would try my sanity. 
### Downsides
This brings about a high cost:
1. Multi-language development is always highly painful. Much of the benefit of tooling in any one language is lost. It's difficult to ensure the consistency of the shape of data and method calls
2. The high complexity necessarily means a higher number of bugs and performance issues than the simplest possible solution that would address my problem. For instance, I could give up on my desire for cross-platform use and just make an app that leans into `SwiftUI` and `CoreAudio` with much less surface area for bugs.
3. Embedding is really annoying. For instance, I can't share evented-networking between `rust` and `swift` easily, so I'll have to decide which one I want to rely more on when downloading assets.
4. Not all problems are best solved with event sourcing. For instance, the client -> server heartbeat that might relay current seek time of a podcast episode presents an extremely high cost for continual logging, but an extremely low cost as a thirty-second updated variable where no history is stored at all.

## MVP Goals
1. I want to subscribe to a podcast feed via one device. I want to see this feed show up on both this device and any other device.
2. I want to see whether or not an episode has been played and how much of it has been played, and have this data be synced across devices soon after that part of the podcast is played. Specifically, within 30 seconds of playing something on one device, I want to see that register on a second device. __It is not clear what should happen if I play on two devices.__
3. I want to browse my existing feeds by list or by album artwork.
4. I want to look at feed and episode notes, whether in html or in text.
5. I want to be able to mark episodes as "archived" or "hidden".

Note what is not in this MVP:
1. No play queue.
2. No playlist/smart feeds/whatever.
3. No second major UI or frontend language.
4. No polling; no periodic updates.
5. No settings or preferences.
6. No authentication.
7. No downloading of episodes—streaming only.
8. No offline functionality to sync—it's either server- or client-driven, no in-between yet.

### MVP Checklist
[x] `commit-store` grpc server
[ ] `commit-store` web frontend
[x] `commit-store` Mac/iOS frontend
[x] `GraphQL` server - add feed
[ ] `GraphQL` server - subscriptions
[x] Juniper -> apollo `NetworkTransport` binding + backing thread-pool on start.
[ ] Commit-Store
	[x] GRPC server
	[x] appendCommit
	[x] streamCommits
	[x] readCommitRange
	[ ] backwardsScan
	[ ] All stream support
	[x] Mac/iOS frontend
	[ ] web frontend
	[ ] GraphQL frontend (would be nice to return commits for mutations with side-effects and no data to return)
[ ] Application server
	[x] GraphQL server - mutation
	[x] GraphQL server - query
	[ ] GraphQL server - subscribe
	[x] casts-swift generation
[ ] macOS/iOS client
	[x] consume `FetchFeeds` query to drive feed, entry views
	[ ] Actually allow streaming of linked content.
	[ ] Provide compile-time option for in-memory vs endpoint-driven application state.
	[ ] Add `episodeState` to `Entry` model corresponding to `Active`, `Archived`, or `Hidden`.
[ ] Infrastructure/Meta
  [ ] Automated iOS/macOS builds
	[ ] Continuous testing (using source hut builds)
	[ ] Continuous integration with railway deployment (or k8s if that fails) + TLS for production environment.
	[ ] Integration with Sentry
	[ ] Integration with Prometheus
	[ ] redis or memcached Aggregate Cache
	[ ] Automatic documentation generation
	[ ] Clamp down on rust lints.

Horizon

* Authentication — should go down to commit-level.
* Downloading + audio need fundamentally different cross-platform sharing solution than graphql can offer.
* Mirroring all assets (audio, artwork, notes) is something that might be worth paying for.
* Linux client
* CLI client
* File-driven configuration for server
* Syncing offline events and integrating both local- and server- state (big push!)
