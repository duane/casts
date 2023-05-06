[wiki](https://man.sr.ht/~duane/casts/)

`casts` is a personal podcast manager for multi-device use.

Overview of top-level contents

* `commit-store` - A rust grpc service that allows writing, reading ranges, and streaming of commits (events).
* `casts` - A rust GraphQL schema, backed by `commit-store`, that describes the comamnds and queries that make up the application logic of the manager.
* `apollo-codegen-config.json`, `queries`, and `schema.graphqls` - used to generate `casts-swift` swift bindings