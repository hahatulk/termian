//! Legacy workspace templates: saved terminal tabs with their connections,
//! titles, startup snippets, layout and view mode. They are kept in the
//! snapshot form (`foreign.workspaceTemplates`), so a sync with the legacy
//! client keeps every field, including the ones this client does not use.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::split_tree::{Layout, Node, SplitTree};
use crate::vault::{HostId, Vault};

/// Legacy `defaultWorkspaceTemplateName`.
pub const DEFAULT_NAME: &str = "New Workspace Template";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    #[default]
    #[serde(rename = "focusView")]
    Focus,
    #[serde(rename = "splitView")]
    Split,
}

/// What a template terminal connects to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Host(HostId),
    Local,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ref {
    pub local_id: Option<u64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateTerminal {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<Ref>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_snippet: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl TemplateTerminal {
    pub fn new(target: Target) -> Self {
        let (host, protocol) = match target {
            Target::Host(id) => (Some(Ref { local_id: Some(id), extra: Map::new() }), "ssh"),
            Target::Local => (None, "local"),
        };
        Self { id: uuid::Uuid::new_v4().to_string(), label: None, host, protocol: Some(protocol.into()), startup_snippet: None, extra: Map::new() }
    }

    /// What it opens; a host that is gone or an unsupported protocol opens nothing.
    pub fn target(&self, vault: &Vault) -> Option<Target> {
        match (self.protocol.as_deref(), self.host.as_ref().and_then(|h| h.local_id)) {
            (Some("local"), _) => Some(Target::Local),
            (None | Some("ssh"), Some(id)) if vault.host(id).is_some() => Some(Target::Host(id)),
            _ => None,
        }
    }

    fn same_target(&self, other: &Self) -> bool {
        self.protocol == other.protocol && self.host.as_ref().and_then(|h| h.local_id) == other.host.as_ref().and_then(|h| h.local_id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TemplateNode {
    pub id: String,
    pub layout: Layout,
    pub size: f32,
    #[serde(default)]
    pub children: Vec<TemplateNode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateLayout {
    pub layout_root: TemplateNode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTemplate {
    #[serde(rename = "local_id")]
    pub id: u64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub view_mode: ViewMode,
    pub workspace_terminals: Vec<TemplateTerminal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_layout: Option<TemplateLayout>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One pane of an open tab, as a template sees it.
pub struct OpenTerminal {
    pub pane: u64,
    pub target: Target,
    /// The template terminal the pane was opened from.
    pub from: Option<String>,
}

impl WorkspaceTemplate {
    /// Legacy `createTemplateFromWorkspace`: the tab's supported panes in
    /// order, the layout without the others. `None` without such panes.
    pub fn from_tab(id: u64, label: &str, view_mode: ViewMode, panes: &[OpenTerminal], tree: &SplitTree, previous: Option<&WorkspaceTemplate>) -> Option<Self> {
        if panes.is_empty() {
            return None;
        }
        let terminals: Vec<TemplateTerminal> = panes
            .iter()
            .map(|pane| {
                let kept = pane.from.as_ref().and_then(|from| previous?.workspace_terminals.iter().find(|t| &t.id == from)).cloned();
                let fresh = TemplateTerminal::new(pane.target);
                match kept {
                    Some(old) if old.same_target(&fresh) => old,
                    _ => fresh,
                }
            })
            .collect();
        let mut tree = tree.clone();
        for leaf in tree.leaves() {
            if !panes.iter().any(|p| p.pane == leaf) {
                tree.remove(leaf);
            }
        }
        let names: HashMap<u64, String> = panes.iter().zip(&terminals).map(|(p, t)| (p.pane, t.id.clone())).collect();
        fn convert(node: &Node, names: &HashMap<u64, String>) -> TemplateNode {
            TemplateNode {
                id: names.get(&node.id).cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                layout: node.layout,
                size: node.size,
                children: node.children.iter().map(|c| convert(c, names)).collect(),
            }
        }
        let label = if label.trim().is_empty() { DEFAULT_NAME.to_owned() } else { label.to_owned() };
        let extra = previous.map(|p| p.extra.clone()).unwrap_or_default();
        Some(Self { id, label, view_mode, workspace_layout: Some(TemplateLayout { layout_root: convert(&tree.root, &names) }), workspace_terminals: terminals, extra })
    }

    /// The layout for the panes opened from it (`template terminal id` →
    /// pane); terminals that were not opened are left out, a template
    /// without a layout gets the legacy balanced one.
    pub fn tree(&self, panes: &[(String, u64)]) -> Option<SplitTree> {
        let pane = |id: &str| panes.iter().find(|(t, _)| t == id).map(|(_, p)| *p);
        let Some(root) = self.workspace_layout.as_ref().map(|l| &l.layout_root) else {
            let ids: Vec<u64> = self.workspace_terminals.iter().filter_map(|t| pane(&t.id)).collect();
            return (!ids.is_empty()).then(|| SplitTree::balanced(&ids));
        };
        let mut next_split = SplitTree::SPLIT_IDS;
        let mut next_missing = 1u64 << 61;
        let mut missing = Vec::new();
        fn walk(node: &TemplateNode, pane: &dyn Fn(&str) -> Option<u64>, next_split: &mut u64, next_missing: &mut u64, missing: &mut Vec<u64>) -> Node {
            if node.children.is_empty() {
                let id = pane(&node.id).unwrap_or_else(|| {
                    *next_missing += 1;
                    missing.push(*next_missing);
                    *next_missing
                });
                return Node { id, layout: Layout::Single, size: node.size, children: Vec::new() };
            }
            *next_split += 1;
            let id = *next_split;
            Node { id, layout: node.layout, size: node.size, children: node.children.iter().map(|c| walk(c, pane, next_split, next_missing, missing)).collect() }
        }
        let root = walk(root, &pane, &mut next_split, &mut next_missing, &mut missing);
        let mut tree = SplitTree::from_root(root);
        let opened = tree.leaves().len() - missing.len();
        if opened == 0 {
            return None;
        }
        for id in missing {
            tree.remove(id);
        }
        // Panes the layout does not mention go beside the last one.
        for (_, pane) in panes {
            if !tree.has(*pane) {
                let last = *tree.leaves().last().expect("one pane");
                tree.split(last, *pane, Layout::Horizontal, crate::split_tree::Position::End);
            }
        }
        Some(tree)
    }

    /// The tab differs from the template: «Update template» is shown.
    pub fn differs(&self, other: &WorkspaceTemplate) -> bool {
        self.label != other.label
            || self.view_mode != other.view_mode
            || self.workspace_terminals.len() != other.workspace_terminals.len()
            || self.workspace_terminals.iter().zip(&other.workspace_terminals).any(|(a, b)| a.id != b.id || !a.same_target(b))
            || match (&self.workspace_layout, &other.workspace_layout) {
                (Some(a), Some(b)) => !same_shape(&a.layout_root, &b.layout_root),
                (a, b) => a.is_some() != b.is_some(),
            }
    }

    /// «N connections» as in the legacy list.
    pub fn connections(&self) -> String {
        match self.workspace_terminals.len() {
            1 => "1 connection".into(),
            n => format!("{n} connections"),
        }
    }
}

/// Same panes, directions and sizes; split node ids are not compared.
fn same_shape(a: &TemplateNode, b: &TemplateNode) -> bool {
    a.layout == b.layout
        && (a.size - b.size).abs() < 1e-4
        && a.children.len() == b.children.len()
        && (!a.children.is_empty() || a.id == b.id)
        && a.children.iter().zip(&b.children).all(|(x, y)| same_shape(x, y))
}

/// The vault's templates; records this client cannot read are skipped here
/// but stay in the vault.
pub fn templates(vault: &Vault) -> Vec<WorkspaceTemplate> {
    list(vault).iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect()
}

fn list(vault: &Vault) -> &[Value] {
    vault.foreign.get("workspaceTemplates").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn list_mut(vault: &mut Vault) -> &mut Vec<Value> {
    if !vault.foreign.is_object() {
        vault.foreign = Value::Object(Map::new());
    }
    let slot = vault.foreign.as_object_mut().expect("object").entry("workspaceTemplates").or_insert_with(|| Value::Array(Vec::new()));
    if !slot.is_array() {
        *slot = Value::Array(Vec::new());
    }
    slot.as_array_mut().expect("array")
}

fn id_of(value: &Value) -> Option<u64> {
    value.get("local_id")?.as_u64()
}

/// An id for a new template.
pub fn next_id(vault: &Vault) -> u64 {
    list(vault).iter().filter_map(id_of).max().unwrap_or(0) + 1
}

/// Adds or replaces a template (by id).
pub fn save(vault: &mut Vault, template: &WorkspaceTemplate) {
    let value = serde_json::to_value(template).expect("serializable");
    let list = list_mut(vault);
    match list.iter_mut().find(|v| id_of(v) == Some(template.id)) {
        Some(slot) => *slot = value,
        None => list.push(value),
    }
}

pub fn remove(vault: &mut Vault, id: u64) {
    list_mut(vault).retain(|v| id_of(v) != Some(id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::split_tree::Position;
    use crate::vault::tests::host;

    fn vault() -> Vault {
        Vault { hosts: vec![host(1, "a", None), host(2, "b", None)], ..Vault::default() }
    }

    #[test]
    fn reads_and_writes_the_legacy_snapshot_form() {
        let mut vault = vault();
        vault.foreign = serde_json::json!({ "workspaceTemplates": [{
            "local_id": 4, "label": "prod", "viewMode": "splitView", "updated_at": "x",
            "workspaceTerminals": [
                { "id": "t1", "host": { "local_id": 1 }, "protocol": "ssh", "label": "web", "startupSnippet": { "script": "cd /srv" } },
                { "id": "t2", "protocol": "local" },
                { "id": "t3", "host": { "local_id": 99 }, "protocol": "ssh" },
                { "id": "t4", "protocol": "session-log" }
            ],
            "workspaceLayout": { "layoutRoot": { "id": "s", "layout": "horizontal", "size": 1, "children": [
                { "id": "t1", "layout": "single", "size": 0.5, "children": [] },
                { "id": "t2", "layout": "single", "size": 0.5, "children": [] }
            ] } }
        }, { "broken": true }] });

        let list = templates(&vault);
        assert_eq!(list.len(), 1, "an unreadable record is skipped");
        let t = &list[0];
        assert_eq!((t.id, t.label.as_str(), t.view_mode, t.connections()), (4, "prod", ViewMode::Split, "4 connections".into()));
        let targets: Vec<_> = t.workspace_terminals.iter().map(|x| x.target(&vault)).collect();
        assert_eq!(targets, [Some(Target::Host(1)), Some(Target::Local), None, None]);

        save(&mut vault, &WorkspaceTemplate { label: "renamed".into(), ..t.clone() });
        let stored = &vault.foreign["workspaceTemplates"];
        assert_eq!(stored.as_array().unwrap().len(), 2, "the unreadable record stays");
        assert_eq!(stored[0]["label"], "renamed");
        assert_eq!(stored[0]["updated_at"], "x", "fields this client does not use are kept");
        assert_eq!(stored[0]["workspaceTerminals"][0]["startupSnippet"]["script"], "cd /srv");
        assert_eq!(next_id(&vault), 5);
        remove(&mut vault, 4);
        assert_eq!(templates(&vault), vec![]);
    }

    #[test]
    fn a_tab_becomes_a_template_and_opens_with_the_same_layout() {
        let mut tree = SplitTree::single(10);
        tree.split(10, 11, Layout::Vertical, Position::End);
        tree.split(11, 12, Layout::Horizontal, Position::End);
        let panes = [
            OpenTerminal { pane: 10, target: Target::Host(1), from: None },
            OpenTerminal { pane: 12, target: Target::Local, from: None },
        ];
        // Pane 11 (a quick connection) cannot be saved.
        let t = WorkspaceTemplate::from_tab(1, "", ViewMode::Split, &panes, &tree, None).unwrap();
        assert_eq!(t.label, DEFAULT_NAME);
        assert_eq!(t.workspace_terminals.len(), 2);

        let opened = [(t.workspace_terminals[0].id.clone(), 20), (t.workspace_terminals[1].id.clone(), 21)];
        let tree = t.tree(&opened).unwrap();
        assert_eq!(tree.leaves(), [20, 21]);
        assert_eq!(tree.root.layout, Layout::Vertical);

        let reopened = [
            OpenTerminal { pane: 20, target: Target::Host(1), from: Some(opened[0].0.clone()) },
            OpenTerminal { pane: 21, target: Target::Local, from: Some(opened[1].0.clone()) },
        ];
        let again = WorkspaceTemplate::from_tab(1, DEFAULT_NAME, ViewMode::Split, &reopened, &tree, Some(&t)).unwrap();
        assert!(!again.differs(&t), "an unchanged tab needs no update");
        let focus = WorkspaceTemplate { view_mode: ViewMode::Focus, ..again.clone() };
        assert!(focus.differs(&t));
        let fewer = WorkspaceTemplate::from_tab(1, DEFAULT_NAME, ViewMode::Split, &reopened[..1], &tree, Some(&t)).unwrap();
        assert!(fewer.differs(&t));
        assert!(WorkspaceTemplate::from_tab(1, "x", ViewMode::Split, &[], &tree, None).is_none());
    }

    #[test]
    fn missing_terminals_leave_the_layout_and_no_layout_is_balanced() {
        let mut t = WorkspaceTemplate {
            id: 1,
            label: "x".into(),
            view_mode: ViewMode::Focus,
            workspace_terminals: vec![TemplateTerminal::new(Target::Local), TemplateTerminal::new(Target::Local), TemplateTerminal::new(Target::Local)],
            workspace_layout: None,
            extra: Map::new(),
        };
        let ids: Vec<String> = t.workspace_terminals.iter().map(|x| x.id.clone()).collect();
        let all = [(ids[0].clone(), 1), (ids[1].clone(), 2), (ids[2].clone(), 3)];
        assert_eq!(t.tree(&all).unwrap(), SplitTree::balanced(&[1, 2, 3]));

        let panes: Vec<OpenTerminal> = all.iter().map(|(id, p)| OpenTerminal { pane: *p, target: Target::Local, from: Some(id.clone()) }).collect();
        t = WorkspaceTemplate::from_tab(1, "x", ViewMode::Focus, &panes, &t.tree(&all).unwrap(), Some(&t)).unwrap();
        let tree = t.tree(&[(ids[0].clone(), 7), (ids[2].clone(), 9)]).unwrap();
        assert_eq!(tree.leaves(), [7, 9]);
        assert!(t.tree(&[]).is_none());
    }
}
