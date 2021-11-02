
use crdts::Orswot;
use rand::seq::SliceRandom;
use rand::{Rng, thread_rng};
use std::sync::Arc;
use thiserror::Error;
use crate::friends;
use crate::names;


/// returned Error for name failures
#[derive(Error, Debug)]
pub enum MutateError {
    /// Error choosing a name
    #[error(transparent)]
    NamesError(#[from] names::NamesError),

    /// Action list index > 100
    #[error("ActionList overflow {0}")]
    ActionListOverflow(usize),

    /// Action list index <>> 100
    #[error("ActionList underflow {0}")]
    ActionListUnderflow(usize),
}

const MIN_SIZE: usize = 10;
const MAX_SIZE: usize = 20;

/// make a change to the map to be broadcast to other Actors
/// if the size of the map is less that minimum, add a key
/// if the size of the map is greater than maximum, delete a key
/// otherwise add a value to a random key
pub fn mutate_map(
    actor_id: usize,
    names: Arc<names::Names>,
    friend_map: &friends::FriendMap,
    action_generator: &ActionGenerator,
) -> Result<crdts::map::Op<String, Orswot<String, usize>, usize>, MutateError> {

    let read_ctx = friend_map.len();
    let len: usize = read_ctx.val;

    let op = if len < MIN_SIZE {
        add_key(actor_id, names, friend_map)?
    } else if len > MAX_SIZE {
        remove_key(actor_id, friend_map)
    } else {
        match action_generator.choose() {
            Action::AddKey => add_key(actor_id, names, friend_map)?,
            Action::AddValue => add_value(actor_id, names, friend_map)?,
            Action::RemoveKey => remove_key(actor_id, friend_map),
            Action::RemoveValue => remove_value(actor_id, friend_map),
        }
    };

    Ok(op)
}

fn add_key(
    actor_id: usize,
    names: Arc<names::Names>,
    friend_map: &friends::FriendMap,
)  -> Result<crdts::map::Op<String, Orswot<String, usize>, usize>, MutateError> {
    let read_ctx = friend_map.len();
    let key = names.choose()?;
    let value = names.choose()?;
    tracing::debug!("Actor {:03}: adding key {} with {}", actor_id, key, value);
    let op = friend_map.update(
        key.as_str(),
        read_ctx.derive_add_ctx(actor_id),
        |set, ctx| set.add(value, ctx),
    );
    Ok(op)
}

fn remove_key(
    actor_id: usize,
    friend_map: &friends::FriendMap,
)  -> crdts::map::Op<String, Orswot<String, usize>, usize> {
    let mut rng = thread_rng();
    let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
    // we unwrap here because we know there more than the minimum keys
    let key = keys.choose(&mut rng).unwrap();
    let val_ctx = friend_map.get(key);
    tracing::debug!("Actor {:03}: removing key {}", actor_id, key);
    friend_map.rm(*key, val_ctx.derive_rm_ctx())
}

fn add_value(
    actor_id: usize,
    names: Arc<names::Names>,
    friend_map: &friends::FriendMap,
)  -> Result<crdts::map::Op<String, Orswot<String, usize>, usize>, MutateError> {
    let mut rng = thread_rng();
    let read_ctx = friend_map.len();
    let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
    let key = keys.choose(&mut rng).unwrap();
    let value = names.choose()?;
    tracing::debug!(
        "Actor {:03}: adding value {} to key {}",
        actor_id,
        value,
        key
    );
    let op = friend_map.update(
        key.as_str(),
        read_ctx.derive_add_ctx(actor_id),
        |set, ctx| set.add(value, ctx),
    );
    Ok(op)
}

/// remove a value if ther is one, otherwise add a value so we have an op
fn remove_value(
    actor_id: usize,
    friend_map: &friends::FriendMap,
)  -> crdts::map::Op<String, Orswot<String, usize>, usize> {
    let mut rng = thread_rng();
    let read_ctx = friend_map.len();
    let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
    // we unwrap here because we know there more than the minimum keys
    let key = keys.choose(&mut rng).unwrap();
    let set_ctx = friend_map.get(key);
    let set = set_ctx.val.unwrap();
    let value = {
        // if 'set' is empty, try to remove "", just so we have an Op
        let items: Vec<_> = set.iter().collect(); 
        match items.choose(&mut rng) {
            Some(val_ctx) => val_ctx.val.clone(),
            None => "".to_string()
        }
    };
    tracing::debug!("Actor {:03}: removing key value {} from {}", actor_id, value, key);
    let op = friend_map.update(
        key.as_str(),
        read_ctx.derive_add_ctx(actor_id),
        |set, _ctx| {
            let element_ctx = set.contains(&value);
            set.rm(value, element_ctx.derive_rm_ctx())
        });

    op
}

/// the action to take when mutating the FriendMap
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// add a new key (with a new value)
    AddKey,

    /// add a new value to an existing key
    AddValue,

    /// remove a key (and all its values)
    RemoveKey,

    /// remove an individual value from an existing key
    RemoveValue,
}

pub struct ActionGenerator {

    /// an array of possible actions
    /// the probability of an action being selected is dtermined by
    /// the nuber of times it occurs in the array 
    actions: [Action; 100],
}

/// a vector linking each map type with its probability (as percentage)
pub type ActionList = Vec<(Action, usize)>;

impl ActionGenerator {

    /// create a new actiongenerator from an ActionList
    /// the probabilities must add up to 100%
    pub fn new(action_list: ActionList) -> Result<Self, MutateError> {
        let mut g = ActionGenerator{ actions: [Action::AddKey; 100]};
        let mut index: usize = 0;
        for (action, probability) in action_list {
            // occupy one element for each 1%
            for _ in 0..probability {
                if index >= 100 {
                    return Err(MutateError::ActionListOverflow(index));
                }
                g.actions[index] = action;
                index += 1;
            }
        }
        if index != 100 {
            return Err(MutateError::ActionListUnderflow(index));
        }
        Ok(g)
    }

    /// select a random action
    fn choose(&self) -> Action {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..100);
        self.actions[index]
    }
}

#[cfg(test)]
mod tests {
    use crdts::{CmRDT, Map};
    use super::*;

    #[test]
    fn can_load_map() {
        let g = ActionGenerator::new(vec![(Action::AddValue, 100)]).unwrap();
        assert_eq!(g.choose(), Action::AddValue);
    }

    #[test]
    fn can_add_key() -> Result<(), MutateError> {
        const ACTOR_ID: usize = 42;
        const NAMES_PATH: &str = "data/names.txt";

        let names = Arc::new(names::Names::new(NAMES_PATH).unwrap());
        let mut friend_map: friends::FriendMap = Map::new();
        assert!(friend_map.is_empty().val);

        let op = add_key(ACTOR_ID, names.clone(), &friend_map)?;
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        Ok(())

    }

    #[test]
    fn can_remove_key() -> Result<(), MutateError> {
        const ACTOR_ID: usize = 42;
        const NAMES_PATH: &str = "data/names.txt";

        let names = Arc::new(names::Names::new(NAMES_PATH).unwrap());
        let mut friend_map: friends::FriendMap = Map::new();
        assert!(friend_map.is_empty().val);

        let op = add_key(ACTOR_ID, names.clone(), &friend_map)?;
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        let op = remove_key(ACTOR_ID, &friend_map);
        friend_map.apply(op);
        assert!(friend_map.is_empty().val);

        Ok(())

    }

    #[test]
    fn can_add_value() -> Result<(), MutateError> {
        const ACTOR_ID: usize = 42;
        const NAMES_PATH: &str = "data/names.txt";

        let names = Arc::new(names::Names::new(NAMES_PATH).unwrap());
        let mut friend_map: friends::FriendMap = Map::new();
        assert!(friend_map.is_empty().val);

        let op = add_key(ACTOR_ID, names.clone(), &friend_map)?;
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        let op = add_value(ACTOR_ID, names.clone(), &friend_map)?;
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        let keys: Vec<_> = friend_map.keys().map(|key_ctx| key_ctx.val).collect();
        assert_eq!(keys.len(), 1);

        let key = keys[0];
        let val = friend_map.get(key).val;
        assert!(!val.is_none());
        if let Some(set) = val{            
            // we should have an item from the original add_key and one
            // from the new add_item
            assert_eq!(set.iter().count(), 2);
        }

        Ok(())

    }

    #[test]
    fn can_remove_value() -> Result<(), MutateError> {
        const ACTOR_ID: usize = 42;
        const NAMES_PATH: &str = "data/names.txt";

        let names = Arc::new(names::Names::new(NAMES_PATH).unwrap());
        let mut friend_map: friends::FriendMap = Map::new();
        assert!(friend_map.is_empty().val);

        let op = add_key(ACTOR_ID, names.clone(), &friend_map)?;
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        // get the keys vec inside this scope so it doesn't stay borrowed 
        let key = {
            let keys: Vec<_> = friend_map.keys().map(|key_ctx| key_ctx.val).collect();
            assert_eq!(keys.len(), 1);
    
            keys[0].clone()
        };

        let val = friend_map.get(&key).val;
        assert!(!val.is_none());
        if let Some(set) = val{            
            // we should have an item from the original add_key
            assert_eq!(set.iter().count(), 1);           
        }

        let op = remove_value(ACTOR_ID, &friend_map);
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        let val = friend_map.get(&key).val;
        assert!(!val.is_none());
        if let Some(set) = val{            
            // we should have removed the item
            assert_eq!(set.iter().count(), 0);           
        }

        // make sure we handle attempting to delete from an empty set
        let op = remove_value(ACTOR_ID, &friend_map);
        friend_map.apply(op);
        assert!(!friend_map.is_empty().val);

        Ok(())

    }

}
