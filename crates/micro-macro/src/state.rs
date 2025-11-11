use crate::actions::{self, Action};
use crate::cache::Cache;
use crate::effects::{self, Effect};
use crate::store::Store;

pub struct State {
    pub store: Store,
    pub cache: Cache,
    action_queue: Vec<Action>,
    effect_queue: Vec<Effect>,
}

impl State {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            cache: Cache::new(),
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
        self.store.ensure_observed_graph_fresh();
    }

    pub fn flush_effects(&mut self) {
        let effects = std::mem::take(&mut self.effect_queue);
        for effect in effects {
            effects::run(&mut self.store, effect);
        }
    }
}
