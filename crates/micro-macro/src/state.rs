use crate::actions::{self, Action};
use crate::effects::{self, Effect};
use crate::store::Store;
use std::ops::{Deref, DerefMut};

pub struct State {
    pub store: Store,
    action_queue: Vec<Action>,
    effect_queue: Vec<Effect>,
}

impl State {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            action_queue: Vec::new(),
            effect_queue: Vec::new(),
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        self.action_queue.push(action);
    }

    pub fn flush_actions(&mut self) {
        let actions = std::mem::take(&mut self.action_queue);
        for action in actions {
            let mut effects =
                actions::update(&mut self.store, action);
            self.effect_queue.append(&mut effects);
        }
    }

    pub fn flush_effects(&mut self) {
        let effects = std::mem::take(&mut self.effect_queue);
        for effect in effects {
            effects::run(&mut self.store, effect);
        }
    }
}

impl Deref for State {
    type Target = Store;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}
