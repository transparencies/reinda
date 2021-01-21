use std::collections::VecDeque;
use ahash::{AHashMap, AHashSet};

use reinda_core::AssetId;


pub(crate) struct DepGraph(AHashMap<AssetId, NodeData>);

#[derive(Default)]
struct NodeData {
    /// List of assets this asset depends on.
    dependencies: AHashSet<AssetId>,

    /// Set of assets that are dependant on this asset.
    rev_dependencies: AHashSet<AssetId>,
}

impl DepGraph {
    pub(crate) fn new() -> Self {
        Self(AHashMap::new())
    }

    /// Adds one edge to this graph: `depender` depends on `dependee`.
    pub(crate) fn add_dependency(&mut self, depender: AssetId, dependee: AssetId) {
        self.0.entry(depender).or_default().dependencies.insert(dependee);
        self.0.entry(dependee).or_default().rev_dependencies.insert(depender);
    }

    /// Returns an iterator over all assets which `asset` directly depends on.
    #[cfg(debug_assertions)] // only used in dev-builds
    pub(crate) fn dependencies_of(&self, asset: AssetId) -> impl '_ + Iterator<Item = AssetId> {
        self.0.get(&asset)
            .map(|data| data.dependencies.iter().copied())
            .into_iter()
            .flatten()
    }

    /// Returns a topological sorting of this dependency graph.
    ///
    /// The first element of the returned list does not have any dependencies.
    /// In general, dependencies can simply be resolved by iterating over the
    /// returned list forwards. If the graph is not a DAG, a vector containing
    /// one cycle is returned.
    pub(crate) fn topological_sort(mut self) -> Result<Vec<AssetId>, Vec<AssetId>> {
        // This is an implementation of Kahn's algorithm.

        let mut queue: VecDeque<_> = self.0.iter()
            .filter(|(_, data)| data.dependencies.is_empty())
            .map(|(id, _)| *id)
            .collect();

        let mut pos = 0;
        while let Some(&depender_id) = queue.get(pos) {
            pos += 1;
            while let Some(dependee_id) = {
                // This is a strange workaround to make the compiler understand
                // the `Drain` iterator can be dropped before the loop body.
                let x = self.node_mut(depender_id).rev_dependencies.drain().next();
                x
            } {
                let dependee = self.node_mut(dependee_id);
                dependee.dependencies.remove(&depender_id);
                if dependee.dependencies.is_empty() {
                    queue.push_back(dependee_id);
                }
            }
        }

        if queue.len() == self.0.len() {
            Ok(queue.into())
        } else {
            // For error reporting, we want to return a cycle here. It is not
            // super cheap, but as it only happens in case of an error, it's
            // fine.
            let (&start_id, _) = self.0.iter()
                .find(|(_, data)| !data.dependencies.is_empty())
                .expect("can't find node with edges, but there should be a cycle");

            let mut out = vec![start_id];
            let mut id = start_id;
            loop {
                // We can just follow one arbitrary edge as all edges now are
                // part of a cycle. However, it might not
                let next = *self.0[&id].dependencies.iter().next().unwrap();
                if let Some(pos) = out.iter().position(|&visited| visited == next) {
                    out.drain(..pos);
                    return Err(out);
                }

                out.push(next);
                id = next;
            }
        }
    }

    fn node_mut(&mut self, id: AssetId) -> &mut NodeData {
        self.0.get_mut(&id).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(u32, u32)]) -> DepGraph {
        let mut g = DepGraph::new();
        for &(from, to) in edges {
            g.add_dependency(AssetId(from), AssetId(to));
        }
        g
    }

    macro_rules! assert_topsort {
        (
            [$($from:literal <- $to:literal),* $(,)?]
            => $res:ident($( [$($id:literal),*] ),* $(,)?)
        ) => {
            let actual = graph(&[$( ($from, $to) ),*]).topological_sort();
            let valid = [
                $( $res(vec![$(AssetId($id)),*]) ),*,
            ];

            if !valid.contains(&actual) {
                panic!("`assert_topsort` failed: {:?} is not in valid solutions: {:#?}", actual, valid);
            }
        };
    }


    #[test]
    fn topological_sort_empty() {
        assert_topsort!([] => Ok([]));
    }

    #[test]
    fn topological_sort_dag() {
        assert_topsort!([0 <- 1] => Ok([1, 0]));
        assert_topsort!([1 <- 0] => Ok([0, 1]));

        assert_topsort!([1 <- 0, 2 <- 1] => Ok([0, 1, 2]));
        assert_topsort!([2 <- 9, 0 <- 2] => Ok([9, 2, 0]));

        assert_topsort!([0 <- 1, 0 <- 2] => Ok([1, 2, 0], [2, 1, 0]));
    }

    #[test]
    fn topological_sort_cycles() {
        assert_topsort!(
            [0 <- 1, 1 <- 2, 2 <- 0, 0 <- 4]
            => Err([0, 1, 2], [1, 2, 0], [2, 0, 1])
        );

        assert_topsort!(
            [
                0 <- 1, 1 <- 2, 2 <- 0,
                1 <- 3, 3 <- 2,
                3 <- 4, 4 <- 5,
            ]
            => Err(
                [0, 1, 2], [1, 2, 0], [2, 0, 1],
                [3, 1, 2], [1, 2, 3], [2, 3, 1],
                [0, 1, 3, 2], [1, 3, 2, 0], [3, 2, 0, 1], [2, 0, 1, 3],
            )
        );
    }
}