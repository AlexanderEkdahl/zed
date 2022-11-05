use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Result};

use sqlez::{
    bindable::{Bind, Column},
    statement::Statement,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceId(Vec<PathBuf>);

impl WorkspaceId {
    pub fn paths(self) -> Vec<PathBuf> {
        self.0
    }
}

impl<P: AsRef<Path>, T: IntoIterator<Item = P>> From<T> for WorkspaceId {
    fn from(iterator: T) -> Self {
        let mut roots = iterator
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        roots.sort();
        Self(roots)
    }
}

impl Bind for &WorkspaceId {
    fn bind(&self, statement: &Statement, start_index: i32) -> Result<i32> {
        bincode::serialize(&self.0)
            .expect("Bincode serialization of paths should not fail")
            .bind(statement, start_index)
    }
}

impl Column for WorkspaceId {
    fn column(statement: &mut Statement, start_index: i32) -> Result<(Self, i32)> {
        let blob = statement.column_blob(start_index)?;
        Ok((WorkspaceId(bincode::deserialize(blob)?), start_index + 1))
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum DockAnchor {
    #[default]
    Bottom,
    Right,
    Expanded,
}

impl Bind for DockAnchor {
    fn bind(&self, statement: &Statement, start_index: i32) -> anyhow::Result<i32> {
        match self {
            DockAnchor::Bottom => "Bottom",
            DockAnchor::Right => "Right",
            DockAnchor::Expanded => "Expanded",
        }
        .bind(statement, start_index)
    }
}

impl Column for DockAnchor {
    fn column(statement: &mut Statement, start_index: i32) -> anyhow::Result<(Self, i32)> {
        String::column(statement, start_index).and_then(|(anchor_text, next_index)| {
            Ok((
                match anchor_text.as_ref() {
                    "Bottom" => DockAnchor::Bottom,
                    "Right" => DockAnchor::Right,
                    "Expanded" => DockAnchor::Expanded,
                    _ => bail!("Stored dock anchor is incorrect"),
                },
                next_index,
            ))
        })
    }
}

pub(crate) type WorkspaceRow = (WorkspaceId, DockAnchor, bool);

#[derive(Debug, PartialEq, Eq)]
pub struct SerializedWorkspace {
    pub dock_anchor: DockAnchor,
    pub dock_visible: bool,
    pub center_group: SerializedPaneGroup,
    pub dock_pane: SerializedPane,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

impl Bind for Axis {
    fn bind(&self, statement: &Statement, start_index: i32) -> anyhow::Result<i32> {
        match self {
            Axis::Horizontal => "Horizontal",
            Axis::Vertical => "Vertical",
        }
        .bind(statement, start_index)
    }
}

impl Column for Axis {
    fn column(statement: &mut Statement, start_index: i32) -> anyhow::Result<(Self, i32)> {
        String::column(statement, start_index).and_then(|(axis_text, next_index)| {
            Ok((
                match axis_text.as_str() {
                    "Horizontal" => Axis::Horizontal,
                    "Vertical" => Axis::Vertical,
                    _ => bail!("Stored serialized item kind is incorrect"),
                },
                next_index,
            ))
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SerializedPaneGroup {
    Group {
        axis: Axis,
        children: Vec<SerializedPaneGroup>,
    },
    Pane(SerializedPane),
}

// Dock panes, and grouped panes combined?
// AND we're collapsing PaneGroup::Pane
// In the case where

impl Default for SerializedPaneGroup {
    fn default() -> Self {
        Self::Group {
            axis: Axis::Horizontal,
            children: vec![Self::Pane(Default::default())],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct SerializedPane {
    pub(crate) children: Vec<SerializedItem>,
}

impl SerializedPane {
    pub fn new(children: Vec<SerializedItem>) -> Self {
        SerializedPane { children }
    }
}

pub type GroupId = i64;
pub type PaneId = i64;
pub type ItemId = usize;

pub(crate) enum SerializedItemKind {
    Editor,
    Diagnostics,
    ProjectSearch,
    Terminal,
}

impl Bind for SerializedItemKind {
    fn bind(&self, statement: &Statement, start_index: i32) -> anyhow::Result<i32> {
        match self {
            SerializedItemKind::Editor => "Editor",
            SerializedItemKind::Diagnostics => "Diagnostics",
            SerializedItemKind::ProjectSearch => "ProjectSearch",
            SerializedItemKind::Terminal => "Terminal",
        }
        .bind(statement, start_index)
    }
}

impl Column for SerializedItemKind {
    fn column(statement: &mut Statement, start_index: i32) -> anyhow::Result<(Self, i32)> {
        String::column(statement, start_index).and_then(|(kind_text, next_index)| {
            Ok((
                match kind_text.as_ref() {
                    "Editor" => SerializedItemKind::Editor,
                    "Diagnostics" => SerializedItemKind::Diagnostics,
                    "ProjectSearch" => SerializedItemKind::ProjectSearch,
                    "Terminal" => SerializedItemKind::Terminal,
                    _ => bail!("Stored serialized item kind is incorrect"),
                },
                next_index,
            ))
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SerializedItem {
    Editor { item_id: usize, path: Arc<Path> },
    Diagnostics { item_id: usize },
    ProjectSearch { item_id: usize, query: String },
    Terminal { item_id: usize },
}

impl SerializedItem {
    pub fn item_id(&self) -> usize {
        match self {
            SerializedItem::Editor { item_id, .. } => *item_id,
            SerializedItem::Diagnostics { item_id } => *item_id,
            SerializedItem::ProjectSearch { item_id, .. } => *item_id,
            SerializedItem::Terminal { item_id } => *item_id,
        }
    }

    pub(crate) fn kind(&self) -> SerializedItemKind {
        match self {
            SerializedItem::Editor { .. } => SerializedItemKind::Editor,
            SerializedItem::Diagnostics { .. } => SerializedItemKind::Diagnostics,
            SerializedItem::ProjectSearch { .. } => SerializedItemKind::ProjectSearch,
            SerializedItem::Terminal { .. } => SerializedItemKind::Terminal,
        }
    }
}

#[cfg(test)]
mod tests {
    use sqlez::connection::Connection;

    use crate::model::DockAnchor;

    use super::WorkspaceId;

    #[test]
    fn test_workspace_round_trips() {
        let db = Connection::open_memory("workspace_id_round_trips");

        db.exec(indoc::indoc! {"
            CREATE TABLE workspace_id_test(
                workspace_id BLOB,
                dock_anchor TEXT
            );"})
            .unwrap();

        let workspace_id: WorkspaceId = WorkspaceId::from(&["\test2", "\test1"]);

        db.prepare("INSERT INTO workspace_id_test(workspace_id, dock_anchor) VALUES (?,?)")
            .unwrap()
            .with_bindings((&workspace_id, DockAnchor::Bottom))
            .unwrap()
            .exec()
            .unwrap();

        assert_eq!(
            db.prepare("SELECT workspace_id, dock_anchor FROM workspace_id_test LIMIT 1")
                .unwrap()
                .row::<(WorkspaceId, DockAnchor)>()
                .unwrap(),
            (WorkspaceId::from(&["\test1", "\test2"]), DockAnchor::Bottom)
        );
    }
}
