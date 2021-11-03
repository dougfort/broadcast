use crdts::{Map, Orswot};

/// uniquely identify each process as an [Actor](https://docs.rs/crdts/7.0.0/crdts/trait.Actor.html)
pub type ActorId = usize;

pub type Name = String;

/// Map from a Name to a Set of the Names of their friends
/// [Map](https://docs.rs/crdts/7.0.0/crdts/map/struct.Map.html) is a CRDT
/// [Orswot](https://docs.rs/crdts/7.0.0/crdts/orswot/struct.Orswot.html) is an add-biased or-set without tombstones
pub type FriendMap = Map<Name, Orswot<Name, ActorId>, ActorId>;
