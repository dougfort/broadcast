use crdts::{Map, Orswot};

pub type ActorId = usize;
pub type FriendMap = Map<String, Orswot<String, ActorId>, ActorId>;
