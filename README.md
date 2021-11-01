# broadcast

## Synopsis

More experiments with [Rust CRDTs](https://github.com/rust-crdt/rust-crdt) using [Tokio](https://tokio.rs/) .

This time I'm trying out [tokio::sync::broadcast](https://docs.rs/tokio/1.13.0/tokio/sync/broadcast/index.html) to simulate a network.

previous [experiment](https://github.com/dougfort/crdt-genome)

## Background

Exploring some ideas of [Martin Kleppmann](https://martin.kleppmann.com), particularly [Local-First software](https://martin.kleppmann.com/papers/local-first.pdf).

Using a [CRDT Map](https://docs.rs/crdts/7.0.0/crdts/map/struct.Map.html) to exercise [CvRDT State Based Replication](https://docs.rs/crdts/7.0.0/crdts/trait.CvRDT.html).

This is an extension of the [reset remove example](https://github.com/rust-crdt/rust-crdt/blob/master/examples/reset_remove.rs) in the [rust-crdt](https://github.com/rust-crdt/rust-crdt) repository.

## Scenario

We spawn [asynchronous tasks](https://docs.rs/tokio/0.2.18/tokio/fn.spawn.html) to represent [Actors](https://docs.rs/crdts/7.0.0/crdts/trait.Actor.html)

Actors subscribe to a [broadcast](https://docs.rs/tokio/1.13.0/tokio/sync/broadcast/index.html)

The Actors map a person's name to the [set](https://docs.rs/crdts/7.0.0/crdts/orswot/index.html)  of the names of their friends.

```rustlang
type ActorId = usize;
type FriendMap = Map<String, Orswot<String, ActorId>, ActorId>;
```

We choose names at random from [this list](https://www.usna.edu/Users/cs/roche/courses/s15si335/proj1/files.php%3Ff=names.txt.html)
