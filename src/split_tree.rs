//! The legacy `SplitViewLayoutTree`: a workspace tab's panes as a tree of
//! nodes. A leaf (`Single`) is a terminal; a split lays its children out
//! side by side (`Horizontal`) or stacked (`Vertical`), each child taking a
//! `size` share of its parent. Ids are the terminals' ids for leaves.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Single,
    Horizontal,
    Vertical,
}

/// Where the new pane goes next to the split one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: u64,
    pub layout: Layout,
    pub size: f32,
    pub children: Vec<Node>,
}

impl Node {
    fn leaf(id: u64, size: f32) -> Self {
        Self { id, layout: Layout::Single, size, children: Vec::new() }
    }
}

/// A pane's place in the tab, as fractions of its width and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitTree {
    pub root: Node,
    /// Ids for split nodes; leaves use the terminals' ids.
    next_split: u64,
}

const SPLIT_IDS: u64 = 1 << 62;

impl SplitTree {
    pub fn single(id: u64) -> Self {
        Self { root: Node::leaf(id, 1.), next_split: SPLIT_IDS }
    }

    /// A tree built elsewhere; split ids must be at least `SPLIT_IDS`.
    pub fn from_root(root: Node) -> Self {
        fn max_split(node: &Node) -> u64 {
            node.children.iter().map(max_split).max().unwrap_or(0).max(if node.children.is_empty() { 0 } else { node.id })
        }
        let next_split = max_split(&root).max(SPLIT_IDS);
        Self { root, next_split }
    }

    /// The first id free for a split node.
    pub const SPLIT_IDS: u64 = SPLIT_IDS;

    fn split_id(&mut self) -> u64 {
        self.next_split += 1;
        self.next_split
    }

    /// Path from the root to the node with `id`, inclusive.
    fn path(&self, id: u64) -> Option<Vec<usize>> {
        fn walk(node: &Node, id: u64, path: &mut Vec<usize>) -> bool {
            if node.id == id {
                return true;
            }
            for (i, child) in node.children.iter().enumerate() {
                path.push(i);
                if walk(child, id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut path = Vec::new();
        walk(&self.root, id, &mut path).then_some(path)
    }

    fn node_mut(&mut self, path: &[usize]) -> &mut Node {
        let mut node = &mut self.root;
        for &i in path {
            node = &mut node.children[i];
        }
        node
    }

    pub fn has(&self, id: u64) -> bool {
        self.path(id).is_some()
    }

    /// Puts pane `new` beside pane `target`, as legacy `split`: in a split of
    /// the same direction it shares `target`'s space; otherwise `target`
    /// becomes a split of the two.
    pub fn split(&mut self, target: u64, new: u64, layout: Layout, position: Position) {
        let Some(path) = self.path(target) else { return };
        if !self.node_mut(&path).children.is_empty() || layout == Layout::Single {
            return;
        }
        let pair = |first: Node, second: Node| match position {
            Position::Start => vec![second, first],
            Position::End => vec![first, second],
        };
        if path.is_empty() {
            let old = Node::leaf(target, 0.5);
            let id = self.split_id();
            self.root = Node { id, layout, size: 1., children: pair(old, Node::leaf(new, 0.5)) };
            return;
        }
        let (parent_path, index) = (&path[..path.len() - 1], path[path.len() - 1]);
        let parent_layout = self.node_mut(parent_path).layout;
        if parent_layout != layout {
            let id = self.split_id();
            let node = self.node_mut(&path);
            node.id = id;
            node.layout = layout;
            node.children = pair(Node::leaf(target, 0.5), Node::leaf(new, 0.5));
            return;
        }
        let parent = self.node_mut(parent_path);
        let half = parent.children[index].size / 2.;
        parent.children[index].size = half;
        let at = if position == Position::Start { index } else { index + 1 };
        parent.children.insert(at, Node::leaf(new, half));
    }

    /// Takes a pane out; its space goes to the others, and a split left with
    /// one child is replaced by it (legacy `remove`).
    pub fn remove(&mut self, id: u64) {
        let Some(path) = self.path(id) else { return };
        if path.is_empty() {
            return;
        }
        let (parent_path, index) = (path[..path.len() - 1].to_vec(), path[path.len() - 1]);
        let parent = self.node_mut(&parent_path);
        let freed = parent.children.remove(index).size;
        if parent.children.len() > 1 {
            let share = freed / parent.children.len() as f32;
            for child in &mut parent.children {
                child.size += share;
            }
            return;
        }
        let mut only = parent.children.remove(0);
        let size = parent.size;
        if let Some((grand_index, grand_path)) = parent_path.split_last() {
            let grand = self.node_mut(grand_path);
            if grand.layout == only.layout && !only.children.is_empty() {
                // The survivor's children join the grandparent in its place.
                for child in &mut only.children {
                    child.size *= size;
                }
                grand.children.splice(*grand_index..=*grand_index, only.children);
                return;
            }
        }
        only.size = size;
        *self.node_mut(&parent_path) = only;
    }

    /// New shares for the children of split `id` (dragging a divider).
    pub fn resize(&mut self, id: u64, sizes: &[f32]) {
        let Some(path) = self.path(id) else { return };
        let node = self.node_mut(&path);
        if sizes.len() == node.children.len() {
            for (child, size) in node.children.iter_mut().zip(sizes) {
                child.size = *size;
            }
        }
    }

    /// Pane ids, left to right and top to bottom.
    pub fn leaves(&self) -> Vec<u64> {
        fn walk(node: &Node, out: &mut Vec<u64>) {
            if node.children.is_empty() {
                out.push(node.id);
            }
            for child in &node.children {
                walk(child, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    /// Every node's place, the root covering 0..1 both ways.
    pub fn rects(&self) -> Vec<(u64, bool, Rect)> {
        fn walk(node: &Node, rect: Rect, out: &mut Vec<(u64, bool, Rect)>) {
            out.push((node.id, node.children.is_empty(), rect));
            let mut offset = 0.;
            for child in &node.children {
                let child_rect = match node.layout {
                    Layout::Vertical => {
                        let height = (rect.bottom - rect.top) * child.size;
                        Rect { top: rect.top + offset, bottom: rect.top + offset + height, ..rect }
                    }
                    _ => {
                        let width = (rect.right - rect.left) * child.size;
                        Rect { left: rect.left + offset, right: rect.left + offset + width, ..rect }
                    }
                };
                offset += match node.layout {
                    Layout::Vertical => child_rect.bottom - child_rect.top,
                    _ => child_rect.right - child_rect.left,
                };
                walk(child, child_rect, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, Rect { left: 0., top: 0., right: 1., bottom: 1. }, &mut out);
        out
    }

    /// Every split: its id, direction, place and its children's shares.
    pub fn splits(&self) -> Vec<(u64, Layout, Rect, Vec<f32>)> {
        fn find(node: &Node, id: u64) -> Option<&Node> {
            if node.id == id {
                return Some(node);
            }
            node.children.iter().find_map(|c| find(c, id))
        }
        self.rects()
            .into_iter()
            .filter(|(_, leaf, _)| !leaf)
            .filter_map(|(id, _, rect)| find(&self.root, id).map(|n| (id, n.layout, rect, n.children.iter().map(|c| c.size).collect())))
            .collect()
    }

    /// The pane beside `id` in `direction` that shares the longest edge with
    /// it (legacy `findAdjacentLeaf`).
    pub fn adjacent(&self, id: u64, direction: Direction) -> Option<u64> {
        const EPS: f32 = 1e-4;
        let rects = self.rects();
        let (_, _, from) = *rects.iter().find(|(node, leaf, _)| *node == id && *leaf)?;
        let overlap = |a: (f32, f32), b: (f32, f32)| (a.1.min(b.1) - a.0.max(b.0)).max(0.);
        rects
            .iter()
            .filter(|(node, leaf, _)| *leaf && *node != id)
            .filter_map(|(node, _, to)| {
                let rank = match direction {
                    Direction::Up => ((from.top - to.bottom).abs() < EPS).then(|| overlap((from.left, from.right), (to.left, to.right))),
                    Direction::Down => ((from.bottom - to.top).abs() < EPS).then(|| overlap((from.left, from.right), (to.left, to.right))),
                    Direction::Left => ((from.left - to.right).abs() < EPS).then(|| overlap((from.top, from.bottom), (to.top, to.bottom))),
                    Direction::Right => ((from.right - to.left).abs() < EPS).then(|| overlap((from.top, from.bottom), (to.top, to.bottom))),
                }?;
                (rank > EPS).then_some((*node, rank))
            })
            // lodash `maxBy`: the first of equal ranks.
            .fold(None, |best: Option<(u64, f32)>, item| if best.is_some_and(|b| b.1 >= item.1) { best } else { Some(item) })
            .map(|(node, _)| node)
    }

    /// Legacy `buildBalancedTree`: panes in pairs, alternating directions,
    /// three left over sharing one split (opening a snippet's targets).
    pub fn balanced(ids: &[u64]) -> Self {
        let mut tree = Self::single(ids.first().copied().unwrap_or(0));
        if ids.len() < 2 {
            return tree;
        }
        let mut level: Vec<Node> = ids.iter().map(|id| Node::leaf(*id, 0.5)).collect();
        let mut layout = Layout::Horizontal;
        while level.len() > 2 {
            let count = level.len();
            let mut next = Vec::new();
            let mut nodes = level.into_iter().peekable();
            let mut i = 0;
            while nodes.peek().is_some() {
                let left = count - i;
                if left == 3 {
                    let children = nodes.by_ref().take(3).map(|n| Node { size: 1. / 3., ..n }).collect();
                    next.push(Node { id: tree.split_id(), layout, size: if i == 0 { 1. } else { 0.5 }, children });
                    break;
                } else if left == 1 {
                    let node = nodes.next().expect("one left");
                    next.push(Node { size: 0.5, ..node });
                    i += 1;
                } else {
                    let children = nodes.by_ref().take(2).map(|n| Node { size: 0.5, ..n }).collect();
                    next.push(Node { id: tree.split_id(), layout, size: 0.5, children });
                    i += 2;
                }
            }
            layout = if layout == Layout::Horizontal { Layout::Vertical } else { Layout::Horizontal };
            level = next;
        }
        if level.len() == 1 {
            tree.root = Node { size: 1., ..level.remove(0) };
        } else {
            let id = tree.split_id();
            tree.root = Node { id, layout, size: 1., children: level };
        }
        tree
    }
}

/// Shares after dragging the divider after child `index` by `delta` (a
/// fraction of the split); no pane gets smaller than `min`.
pub fn dragged(sizes: &[f32], index: usize, delta: f32, min: f32) -> Vec<f32> {
    let mut out = sizes.to_vec();
    if index + 1 >= out.len() {
        return out;
    }
    let pair = out[index] + out[index + 1];
    let first = (out[index] + delta).clamp(min.min(pair / 2.), (pair - min).max(pair / 2.));
    out[index] = first;
    out[index + 1] = pair - first;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shares(tree: &SplitTree) -> Vec<(u64, f32, f32, f32, f32)> {
        tree.rects().into_iter().filter(|(_, leaf, _)| *leaf).map(|(id, _, r)| (id, r.left, r.top, r.right, r.bottom)).collect()
    }

    #[test]
    fn splits_share_space_and_nest_across_directions() {
        let mut tree = SplitTree::single(1);
        tree.split(1, 2, Layout::Horizontal, Position::End);
        assert_eq!(shares(&tree), [(1, 0., 0., 0.5, 1.), (2, 0.5, 0., 1., 1.)]);
        tree.split(2, 3, Layout::Horizontal, Position::Start);
        assert_eq!(tree.leaves(), [1, 3, 2], "same direction: a third pane in the row");
        assert_eq!(shares(&tree)[1], (3, 0.5, 0., 0.75, 1.));
        tree.split(1, 4, Layout::Vertical, Position::End);
        assert_eq!(shares(&tree)[..2], [(1, 0., 0., 0.5, 0.5), (4, 0., 0.5, 0.5, 1.)], "a stacked pair where pane 1 was");
        assert!(tree.has(4) && !tree.has(9));
    }

    #[test]
    fn removing_gives_space_back_and_collapses_splits() {
        let mut tree = SplitTree::single(1);
        tree.split(1, 2, Layout::Horizontal, Position::End);
        tree.split(2, 3, Layout::Horizontal, Position::End);
        tree.remove(3);
        assert_eq!(shares(&tree), [(1, 0., 0., 0.625, 1.), (2, 0.625, 0., 1., 1.)], "freed space is shared equally, as in legacy");
        tree.split(2, 4, Layout::Vertical, Position::End);
        tree.remove(4);
        assert_eq!(shares(&tree), [(1, 0., 0., 0.625, 1.), (2, 0.625, 0., 1., 1.)], "a split of one becomes its pane");
        tree.remove(1);
        assert_eq!(tree.root, Node::leaf(2, 1.));
        tree.remove(2);
        assert_eq!(tree.leaves(), [2], "the last pane stays");
    }

    #[test]
    fn a_survivor_split_joins_a_grandparent_of_its_direction() {
        // [1 | [2 / [3 | 4]]]; removing 2 leaves [3 | 4] inside the row.
        let mut tree = SplitTree::single(1);
        tree.split(1, 2, Layout::Horizontal, Position::End);
        tree.split(2, 3, Layout::Vertical, Position::End);
        tree.split(3, 4, Layout::Horizontal, Position::End);
        tree.remove(2);
        assert_eq!(shares(&tree), [(1, 0., 0., 0.5, 1.), (3, 0.5, 0., 0.75, 1.), (4, 0.75, 0., 1., 1.)]);
    }

    #[test]
    fn resize_and_neighbours() {
        let mut tree = SplitTree::single(1);
        tree.split(1, 2, Layout::Horizontal, Position::End);
        tree.split(2, 3, Layout::Vertical, Position::End);
        let root = tree.root.id;
        tree.resize(root, &[0.25, 0.75]);
        assert_eq!(shares(&tree)[0], (1, 0., 0., 0.25, 1.));
        assert_eq!(tree.adjacent(1, Direction::Right), Some(2), "the longer shared edge wins a tie by order");
        assert_eq!(tree.adjacent(3, Direction::Up), Some(2));
        assert_eq!(tree.adjacent(3, Direction::Left), Some(1));
        assert_eq!(tree.adjacent(1, Direction::Left), None);
    }

    #[test]
    fn dragging_a_divider_moves_space_between_its_neighbours() {
        assert_eq!(dragged(&[0.5, 0.5], 0, 0.2, 0.1), [0.7, 0.3]);
        assert_eq!(dragged(&[0.5, 0.5], 0, 0.6, 0.1), [0.9, 0.100000024], "the other pane keeps its minimum");
        assert_eq!(dragged(&[0.25, 0.25, 0.5], 1, -0.2, 0.1), [0.25, 0.1, 0.65]);
        assert_eq!(dragged(&[1.], 0, 0.3, 0.1), [1.]);
        let mut tree = SplitTree::single(1);
        tree.split(1, 2, Layout::Vertical, Position::End);
        let splits = tree.splits();
        assert_eq!((splits.len(), splits[0].1, splits[0].3.clone()), (1, Layout::Vertical, vec![0.5, 0.5]));
    }

    #[test]
    fn balanced_trees_follow_legacy() {
        assert_eq!(SplitTree::balanced(&[1]).leaves(), [1]);
        let two = SplitTree::balanced(&[1, 2]);
        assert_eq!((two.root.layout, two.leaves()), (Layout::Horizontal, vec![1, 2]));
        let three = SplitTree::balanced(&[1, 2, 3]);
        assert_eq!(three.root.children.len(), 3, "three share one split");
        let four = SplitTree::balanced(&[1, 2, 3, 4]);
        assert_eq!((four.root.layout, four.leaves()), (Layout::Vertical, vec![1, 2, 3, 4]));
        assert_eq!(shares(&four)[0], (1, 0., 0., 0.5, 0.5));
        let five = SplitTree::balanced(&[1, 2, 3, 4, 5]);
        assert_eq!(five.leaves(), [1, 2, 3, 4, 5]);
        let total: f32 = shares(&five).iter().map(|(_, l, t, r, b)| (r - l) * (b - t)).sum();
        assert!((total - 1.).abs() < 1e-4, "the panes fill the tab: {total}");
    }
}
