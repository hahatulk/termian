//! Tab strip model: pinned Vaults/SFTP tabs plus closable session tabs.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabKind {
    Vaults,
    Sftp,
    Session { title: String },
    NewTab,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: u64,
    pub kind: TabKind,
}

impl Tab {
    pub fn title(&self) -> &str {
        match &self.kind {
            TabKind::Vaults => "Vaults",
            TabKind::Sftp => "SFTP",
            TabKind::Session { title } => title,
            TabKind::NewTab => "New Tab",
        }
    }

    pub fn closable(&self) -> bool {
        !matches!(self.kind, TabKind::Vaults | TabKind::Sftp)
    }
}

#[derive(Debug)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active: u64,
    next_id: u64,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            tabs: vec![
                Tab { id: 0, kind: TabKind::Vaults },
                Tab { id: 1, kind: TabKind::Sftp },
            ],
            active: 0,
            next_id: 2,
        }
    }
}

impl Tabs {
    pub fn all(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active(&self) -> &Tab {
        self.tabs.iter().find(|t| t.id == self.active).expect("active tab exists")
    }

    pub fn open(&mut self, kind: TabKind) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab { id, kind });
        self.active = id;
        id
    }

    pub fn activate(&mut self, id: u64) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active = id;
        }
    }

    /// Closing the active tab activates its left neighbour; pinned tabs ignore close.
    pub fn close(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else { return };
        if !self.tabs[index].closable() {
            return;
        }
        self.tabs.remove(index);
        if self.active == id {
            self.active = self.tabs[index - 1].id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_pinned_vaults_and_sftp() {
        let tabs = Tabs::default();
        let titles: Vec<_> = tabs.all().iter().map(Tab::title).collect();
        assert_eq!(titles, ["Vaults", "SFTP"]);
        assert_eq!(tabs.active().title(), "Vaults");
    }

    #[test]
    fn opening_a_tab_activates_it() {
        let mut tabs = Tabs::default();
        tabs.open(TabKind::Session { title: "Termian".into() });
        assert_eq!(tabs.active().title(), "Termian");
    }

    #[test]
    fn closing_active_tab_activates_left_neighbour() {
        let mut tabs = Tabs::default();
        tabs.open(TabKind::Session { title: "Termian".into() });
        let new_tab = tabs.open(TabKind::NewTab);
        tabs.close(new_tab);
        assert_eq!(tabs.active().title(), "Termian");
        assert_eq!(tabs.all().len(), 3);
    }

    #[test]
    fn pinned_tabs_cannot_be_closed() {
        let mut tabs = Tabs::default();
        tabs.close(0);
        tabs.close(1);
        assert_eq!(tabs.all().len(), 2);
    }

    #[test]
    fn closing_inactive_tab_keeps_active() {
        let mut tabs = Tabs::default();
        let a = tabs.open(TabKind::Session { title: "a".into() });
        tabs.open(TabKind::Session { title: "b".into() });
        tabs.close(a);
        assert_eq!(tabs.active().title(), "b");
    }
}
