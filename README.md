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

The Actors maintain a map of a person's name to the [set](https://docs.rs/crdts/7.0.0/crdts/orswot/index.html)  of the names of their friends.

```rustlang
use crdts::{Map, Orswot};

/// uniquely identify each process as an [Actor]
pub type ActorId = usize;

pub type Name = String;

/// Map from a Name to a Set of the Names of their friends
/// [Map] is a CRDT
/// [Orswot] is an add-biased or-set without tombstones
pub type FriendMap = Map<Name, Orswot<Name, ActorId>, ActorId>;
```

Each Actor mutates its map at random intervals and broadscasts the changed map.

The goal is to observe every map instance converging to a common value.

We choose names at random from [this list](https://www.usna.edu/Users/cs/roche/courses/s15si335/proj1/files.php%3Ff=names.txt.html)

## Mutation

At random intervals, each actor selects a random action action based on a probability table.

```rustlang
    // TODO: #2 load the action list from config
    let action_list: mutate::ActionList = vec![
        (mutate::Action::AddKey, 10),
        (mutate::Action::AddValue, 40),   
        (mutate::Action::RemoveKey, 10),
        (mutate::Action::RemoveValue, 40),    
    ];
```

Then it broadcasts the modified Map to all active actors.

## Execution

To run this simulation, simply get the repository and run

```bash
cargo run
```

## Verification

Verification is difficult, becasue the system is never at rest and there is no easy
way to poll the active agents for their current state.

After merging an incoming map, we check that the merged map is equal to the
incoming map.

```rustlang
    trans_count += 1;
    let incoming_map: friends::FriendMap = serde_json::from_str(&incoming_data)?;
    let test_map = incoming_map.clone();
    friend_map.validate_merge(&incoming_map)?;
    friend_map.merge(incoming_map);
    if test_map == friend_map {
        after_count += 1;
    }
    let percentage: f64 = (after_count as f64 / trans_count as f64) * 100.0;
```

We monitor the percentage of cases where the maps are equal. This is not a
perfect test, but it provides some indication. If the percentage never decreases
and maintains a high level, we consider the test acceptable.

```bash
Nov 02 16:22:20.216 DEBUG broadcast::mutate: Actor 009: removing key value Wilson from Cyndal
Nov 02 16:22:20.218 DEBUG broadcast: Actor(001) from Actor(009)   275 of   319 86.21%
Nov 02 16:22:20.218 DEBUG broadcast: Actor(005) from Actor(009)   262 of   306 85.62%
Nov 02 16:22:20.218 DEBUG broadcast: Actor(004) from Actor(009)   259 of   303 85.48%
Nov 02 16:22:20.218 DEBUG broadcast: Actor(003) from Actor(009)   255 of   299 85.28%
Nov 02 16:22:20.218 DEBUG broadcast: Actor(010) from Actor(009)   266 of   310 85.81%
Nov 02 16:22:20.219 DEBUG broadcast: Actor(006) from Actor(009)   281 of   325 86.46%
Nov 02 16:22:20.219 DEBUG broadcast: Actor(008) from Actor(009)   286 of   330 86.67%
Nov 02 16:22:20.219 DEBUG broadcast: Actor(002) from Actor(009)   280 of   324 86.42%
Nov 02 16:22:20.219 DEBUG broadcast: Actor(007) from Actor(009)   263 of   307 85.67%
Nov 02 16:22:22.219 DEBUG broadcast::mutate: Actor 005: removing key value Niasha from Katurah
Nov 02 16:22:22.223 DEBUG broadcast: Actor(003) from Actor(005)   256 of   300 85.33%
Nov 02 16:22:22.223 DEBUG broadcast: Actor(006) from Actor(005)   282 of   326 86.50%
Nov 02 16:22:22.223 DEBUG broadcast: Actor(009) from Actor(005)   275 of   319 86.21%
Nov 02 16:22:22.223 DEBUG broadcast: Actor(002) from Actor(005)   281 of   325 86.46%
Nov 02 16:22:22.223 DEBUG broadcast: Actor(008) from Actor(005)   287 of   331 86.71%
Nov 02 16:22:22.225 DEBUG broadcast: Actor(001) from Actor(005)   276 of   320 86.25%
Nov 02 16:22:22.225 DEBUG broadcast: Actor(007) from Actor(005)   264 of   308 85.71%
Nov 02 16:22:22.225 DEBUG broadcast: Actor(004) from Actor(005)   260 of   304 85.53%
Nov 02 16:22:22.225 DEBUG broadcast: Actor(010) from Actor(005)   267 of   311 85.85%
```
