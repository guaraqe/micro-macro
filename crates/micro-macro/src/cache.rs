use crate::graph_state::calculate_observed_graph;
use crate::graph_view::ObservedGraphDisplay;
use crate::store::Store;
use crate::versioned::Memoized;

pub struct Cache {
    pub observed_graph:
        Memoized<Store, (u64, u64), ObservedGraphDisplay>,
}

impl Cache {
    pub fn new() -> Self {
        let observed_graph = Memoized::new(
            |s: &Store| {
                (
                    s.state_graph_v.version(),
                    s.observable_graph_v.version(),
                )
            },
            |s: &Store| {
                calculate_observed_graph(
                    s.state_graph_v.get(),
                    s.observable_graph_v.get(),
                )
            },
        );
        Self { observed_graph }
    }
}
