extern crate petgraph;
use petgraph::data::*;
use petgraph::dot::*;
use petgraph::graph::*;
use petgraph::visit::*;
use petgraph::*;
use petgraph_evcxr::*;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone)]
pub struct Overlay<G, NOW, EOW>
where
    G: GraphBase + Data + IntoEdgeReferences,
{
    nodes: HashMap<<G as GraphBase>::NodeId, NOW>,
    edges: HashMap<<G as GraphBase>::EdgeId, EOW>,
    edge_refs: HashMap<<G as GraphBase>::EdgeId, <G as IntoEdgeReferences>::EdgeRef>,
    graph: G,
}

#[derive(Debug, Clone)]
enum Phase<'a, N, E, NOW, EOW> {
    Nodes(std::collections::hash_map::Iter<'a, N, NOW>),
    Edges(std::collections::hash_map::Iter<'a, E, EOW>),
}

pub struct OverlayedItems<'a, G, NOW, EOW>
where
    G: GraphBase + Data + DataMap + IntoEdgeReferences,
{
    overlay: &'a Overlay<G, NOW, EOW>,
    phase: Phase<
        'a,
        <<G as IntoEdgeReferences>::EdgeRef as EdgeRef>::NodeId,
        <<G as IntoEdgeReferences>::EdgeRef as EdgeRef>::EdgeId,
        NOW,
        EOW,
    >,
    node_indexes: HashMap<<<G as IntoEdgeReferences>::EdgeRef as EdgeRef>::NodeId, usize>,
}

impl<G, NOW, EOW> Overlay<G, NOW, EOW>
where
    G: GraphBase + Data + DataMap + IntoEdgeReferences,
    <G as GraphBase>::NodeId: Hash + Eq,
    <G as GraphBase>::EdgeId: Hash + Eq,
{
    pub fn overlayed_elements<'b>(&'b self) -> OverlayedItems<'b, G, NOW, EOW> {
        OverlayedItems {
            overlay: self,
            phase: Phase::Nodes(self.nodes.iter()),
            node_indexes: HashMap::new(),
        }
    }
    pub fn overlay_edge<'b>(&'b mut self, edge: <G as IntoEdgeReferences>::EdgeRef, eow: EOW) {
        self.edges.insert(edge.id(), eow);
        self.edge_refs.insert(edge.id(), edge);
    }
    pub fn overlay_node<'b>(&'b mut self, node: <G as GraphBase>::NodeId, now: NOW) {
        self.nodes.insert(node, now);
    }
    pub fn remove_edge<'b>(&'b mut self, edge: <G as GraphBase>::EdgeId) {
        self.edges.remove(&edge);
        self.edge_refs.remove(&edge);
    }
    pub fn remove_node<'b>(&'b mut self, node: <G as GraphBase>::NodeId) {
        self.nodes.remove(&node);
    }
}

impl<G, NOW, EOW> Overlay<G, NOW, EOW>
where
    G: GraphBase
        + Data
        + DataMap
        + GraphProp
        + NodeIndexable
        + IntoNodeReferences
        + IntoEdgeReferences,
    <G as GraphBase>::EdgeId: Copy + Eq + Hash,
    <G as GraphBase>::NodeId: Copy + Eq + Hash,
    <G as Data>::NodeWeight: std::fmt::Display,
    <G as Data>::EdgeWeight: std::fmt::Display,
{
    pub fn draw_overlayed<'b>(&'b self) {
        draw_graph_with_attr_getters(
            self.graph,
            &[],
            &|_, er| {
                format!(
                    "color = {}",
                    if self.edges.contains_key(&er.id()) {
                        "red"
                    } else {
                        "black"
                    }
                )
            },
            &|_, nr| {
                format!(
                    "color = {}",
                    if self.nodes.contains_key(&nr.id()) {
                        "red"
                    } else {
                        "black"
                    }
                )
            },
        );
    }
}

impl<'a, G, NOW, EOW> Iterator for OverlayedItems<'a, G, NOW, EOW>
where
    G: GraphBase + Data + DataMap + IntoEdgeReferences,
    <G as Data>::NodeWeight: Clone,
    <G as Data>::EdgeWeight: Clone,
    <G as GraphBase>::EdgeId: Copy + Eq + Hash,
    <G as GraphBase>::NodeId: Copy + Eq + Hash,
    NOW: Clone,
    EOW: Clone,
{
    type Item = Element<<G as Data>::NodeWeight, <G as Data>::EdgeWeight>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Phase::Nodes(ref mut nodes) = self.phase {
            match nodes.next() {
                Some(node) => {
                    self.node_indexes.insert(*node.0, self.node_indexes.len());
                    return Some(Element::Node {
                        weight: self.overlay.graph.node_weight(*node.0).unwrap().clone(),
                    });
                }
                None => self.phase = Phase::Edges(self.overlay.edges.iter()),
            }
        };
        if let Phase::Edges(ref mut edges) = self.phase.clone() {
            loop {
                let edge_ref_maybe = edges
                    .next()
                    .and_then(|edge| self.overlay.edge_refs.get(edge.0));
                match edge_ref_maybe {
                    Some(edge_ref) => {
                        let new_edge_maybe =
                            self.node_indexes
                                .get(&edge_ref.source())
                                .and_then(|source| {
                                    self.node_indexes.get(&edge_ref.target()).map(|target| {
                                        Element::Edge {
                                            weight: edge_ref.weight().clone(),
                                            source: *source,
                                            target: *target,
                                        }
                                    })
                                });
                        match new_edge_maybe {
                            Some(new_edge) => {
                                self.phase = Phase::Edges(edges.clone());
                                return Some(new_edge);
                            }
                            None => continue,
                        }
                    }
                    None => {
                        self.phase = Phase::Edges(edges.clone());
                        return None;
                    }
                }
            }
        };
        None
    }
}

pub type Selection<G> = Overlay<G, (), ()>;

impl<G> Selection<G>
where
    G: GraphBase + Data + DataMap + IntoEdgeReferences,
    <G as GraphBase>::EdgeId: Copy + Eq + Hash,
    <G as GraphBase>::NodeId: Copy + Eq + Hash,
{
    pub fn new(g: G) -> Selection<G> {
        Selection {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            edge_refs: HashMap::new(),
            graph: g,
        }
    }
    pub fn select_edge<'b>(&'b mut self, edge: <G as IntoEdgeReferences>::EdgeRef) {
        self.overlay_edge(edge, ());
    }
    pub fn select_node<'b>(&'b mut self, node: <G as GraphBase>::NodeId) {
        self.overlay_node(node, ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::*;
    use petgraph::visit::*;
    use petgraph::data::*;
    use petgraph_examples as examples;
    use petgraph::visit::IntoNodeReferences;
    use petgraph::algo::*;

    #[test]
    fn test_selection() {
        let dwc = examples::directed_graph_with_cycle();
        let mut selection = Selection::new(&dwc);
        selection.select_node(node_index(0));
        selection.select_edge(dwc.edge_references().next().unwrap());
        selection.select_node(node_index(1));
        let mut expected_extraction:Graph<String, String, petgraph::Directed> = Graph::new();
        let a = expected_extraction.add_node("a".to_string());
        let b = expected_extraction.add_node("b".to_string());
        let edge = expected_extraction.add_edge(a, b, "".to_string());
        {
            let overlayed_elements = selection.overlayed_elements();
            let extracted_selection: Graph<String, String, petgraph::Directed> = Graph::from_elements(overlayed_elements);
            assert!(is_isomorphic_matching(&extracted_selection, &expected_extraction, |a, b| a == b, |a, b| a==b));
        }
        {
            let mut selection1 = selection.clone();
            selection1.remove_edge(edge_index(0));
            expected_extraction.remove_edge(edge);
            let overlayed_elements = selection1.overlayed_elements();
            let extracted_selection: Graph<String, String, petgraph::Directed> = Graph::from_elements(overlayed_elements);
            assert!(is_isomorphic_matching(&extracted_selection, &expected_extraction, |a, b| a == b, |a, b| a==b));
        }
    }
}
