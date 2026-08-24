`casts` is a personal podcast manager for multi-device use.

The best way to introduce this project is to view it as the confluence of my interest in multiple topics: cross-platform app-data promulgation for a given application, event sourcing and CRDTs, my love of macOS/iOS user interface design, and my daily enjoyment of podcasts.

Simply put, I was frustrated with the difficulty of keeping track of which podcasts I was listening to, which I had already listened to, which I wanted to archive and keep audio data files around for, etc. The existing solutions were either buggy, didn't offer syncing, or only worked on a subset of the idiosyncratic platforms I work on (namely, macOS, Linux, and FreeBSD). Needing a project anyway to keep up my skills, I decided to over-engineer my own solution. The application state is a stream of events in a dedicated commit store; commands validate against an aggregate hydrated from that stream and append new events; and feed refreshes are stored as diffs, so state syncs between nodes as small, mergeable deltas.

## Components

1. The [commit store](https://git.sr.ht/~duane/commit-store) — its own repository, checked out as a **sibling** of this one (`../commit-store`), backed by [postgresql](https://git.sr.ht/~duane/commit-store/tree/main/item/src/integrations/postgres) or [sqlite](https://git.sr.ht/~duane/commit-store/tree/main/item/src/integrations/sqlite). It accepts commits to a given stream and serves long-lived subscriptions that stream updates. Conflicts resolve by enforcing that each commit writes exactly one version on top of the current stream version, or it is rejected. It exports a native Rust API, a gRPC service with mutual TLS, and is deployed on its own.
2. The domain library in [`casts/`](casts/): [commands](casts/src/commands/feed_aggregate_command.rs) validate against the [aggregate](casts/src/aggregate/feed_aggregate.rs) and emit [events](casts/src/events/feed_aggregate_event.rs); [`immutable-feed/`](immutable-feed/) parses RSS/Atom and computes the diffs that `ObservedChanges` events carry. Command application is pure — callers fetch feeds first.
3. A gRPC query/command service, [`casts-grpc-server`](casts/src/bin/casts-grpc-server.rs) (`ListFeeds`, `ListEntries`, `GetFeed`, `RecordPlayback`), and a CLI, [`casts`](casts/src/bin/casts.rs) (`add`, `poll`, `mark-listened`, `list-commits`). A [juniper GraphQL schema](casts/src/graphql/mod.rs) still builds behind the optional `graphql` feature (`schema.graphqls` is its generated SDL; the HTTP server that fronted it is parked pending a hyper 1.x port).
4. A SwiftUI front-end for iOS and macOS in [`casts-ios/`](casts-ios/), which talks to `casts-grpc-server` over grpc-swift and plays episodes with AVPlayer (AirPlay included).

An earlier iteration embedded the GraphQL runtime inside the native app through a C FFI bridge (Apollo iOS calling in-memory into Rust). That path was removed in favor of gRPC; it lives in git history (`casts-c/`, `juniper-swift/`, `casts-ios/casts-swift/`) if it is ever revived.

## Running locally

```
docker compose up db commit-store
docker compose run --rm commit-store provision   # one-shot schema provisioning
cargo run --bin casts-grpc-server                # talks to localhost:7392, serves :8243
cargo run --bin casts -- add https://example.com/feed.xml
```

`COMMIT_STORE_ADDR` points clients at a non-default commit store. The stream id is currently hardcoded; single-user for now.

## Status

Work in progress. Listened-to tracking (`RecordPlayback` / `mark-listened`) lands the write path first; surfacing it in the iOS player UI is next, along with reconnecting the add-feed flow in the app.

For more of a technical look at why I developed this at all, see https://man.sr.ht/~duane/casts. If you have questions, please reach out to duane@duane.cc.
