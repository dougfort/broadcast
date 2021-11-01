use anyhow::Error;
use crdts::Orswot;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::Arc;

use crate::friends;
use crate::names;

/// make a change to the map to be broadcast to other Actors
/// if the size of the map is less that minimum, add a key
/// if the size of the map is greater than maximum, delete a key
/// otherwise add a value to a random key
pub fn mutate_map(
    actor_id: usize,
    names: Arc<names::Names>,
    friend_map: &friends::FriendMap,
) -> Result<crdts::map::Op<String, Orswot<String, usize>, usize>, Error> {
    const MIN_SIZE: usize = 10;
    const MAX_SIZE: usize = 20;

    let read_ctx = friend_map.len();
    let len: usize = read_ctx.val;

    let op = if len < MIN_SIZE {
        let key = names.choose()?;
        let value = names.choose()?;
        tracing::debug!("Actor {:03}: adding key {} with {}", actor_id, key, value);
        friend_map.update(
            key.as_str(),
            read_ctx.derive_add_ctx(actor_id),
            |set, ctx| set.add(value, ctx),
        )
    } else if len > MAX_SIZE {
        let mut rng = thread_rng();
        let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
        let key = keys.choose(&mut rng).unwrap();
        tracing::debug!("Actor {:03}: removing key {}", actor_id, key);
        friend_map.rm(*key, read_ctx.derive_rm_ctx())
    } else {
        let mut rng = thread_rng();
        let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
        let key = keys.choose(&mut rng).unwrap();
        let value = names.choose()?;
        tracing::debug!(
            "Actor {:03}: adding value {} to key {}",
            actor_id,
            value,
            key
        );
        friend_map.update(
            key.as_str(),
            read_ctx.derive_add_ctx(actor_id),
            |set, ctx| set.add(value, ctx),
        )
    };

    Ok(op)
}
