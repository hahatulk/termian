//! Tab strip model: pinned Vaults/SFTP tabs plus closable session tabs.

/// Ordered by creation: a larger id was opened later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabKind {
    Vaults,
    Sftp,
    Session { title: String },
    /// The launcher tab with recent connections and host search.
    Launcher,
}

/// Tabs that always exist, in display order, before any session tab.
const PINNED: [TabKind; 2] = [TabKind::Vaults, TabKind::Sftp];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub kind: TabKind,
}

impl Tab {
    pub fn title(&self) -> &str {
        match &self.kind {
            TabKind::Vaults => "Vaults",
            TabKind::Sftp => "SFTP",
            TabKind::Session { title } => title,
            TabKind::Launcher => "New Tab",
        }
    }

    pub fn closable(&self) -> bool {
        !PINNED.contains(&self.kind)
    }
}

#[derive(Debug)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active: TabId,
    next_id: u64,
}

impl Default for Tabs {
    fn default() -> Self {
        let tabs: Vec<_> = PINNED
            .iter()
            .enumerate()
            .map(|(i, kind)| Tab { id: TabId(i as u64), kind: kind.clone() })
            .collect();
        Self { active: tabs[0].id, next_id: tabs.len() as u64, tabs }
    }
}

impl Tabs {
    pub fn all(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active(&self) -> &Tab {
        self.tabs.iter().find(|t| t.id == self.active).expect("active tab is never removed")
    }

    pub fn open(&mut self, kind: TabKind) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;
        self.tabs.push(Tab { id, kind });
        self.active = id;
        id
    }

    pub fn activate(&mut self, id: TabId) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active = id;
        }
    }

    /// «Next Tab» (`1`) / «Previous Tab» (`-1`), wrapping around.
    pub fn cycle(&mut self, step: isize) {
        let count = self.tabs.len() as isize;
        let at = self.tabs.iter().position(|t| t.id == self.active).unwrap_or(0) as isize;
        self.active = self.tabs[(at + step).rem_euclid(count) as usize].id;
    }

    /// «Switch to specific Tab [1…9]»: the tab at that position, if any.
    pub fn activate_nth(&mut self, number: usize) {
        if let Some(tab) = number.checked_sub(1).and_then(|i| self.tabs.get(i)) {
            self.active = tab.id;
        }
    }

    /// Closing the active tab activates its left neighbour; pinned tabs ignore close.
    /// Renames a session tab.
    pub fn set_title(&mut self, id: TabId, title: &str) {
        if let Some(Tab { kind: TabKind::Session { title: current }, .. }) = self.tabs.iter_mut().find(|t| t.id == id) {
            *current = title.to_owned();
        }
    }

    pub fn close(&mut self, id: TabId) {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else { return };
        if !self.tabs[index].closable() {
            return;
        }
        self.tabs.remove(index);
        if self.active == id {
            // A closable tab always sits after the pinned ones, so index >= 1.
            self.active = self.tabs[index - 1].id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(title: &str) -> TabKind {
        TabKind::Session { title: title.into() }
    }

    #[test]
    fn shortcuts_cycle_and_pick_tabs() {
        let mut tabs = Tabs::default();
        tabs.open(session("a"));
        tabs.cycle(1);
        assert_eq!(tabs.active().title(), "Vaults", "wraps forward");
        tabs.cycle(-1);
        assert_eq!(tabs.active().title(), "a", "wraps back");
        tabs.activate_nth(2);
        assert_eq!(tabs.active().title(), "SFTP");
        tabs.activate_nth(9);
        assert_eq!(tabs.active().title(), "SFTP", "no ninth tab");
    }

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
        tabs.open(session("Termian"));
        assert_eq!(tabs.active().title(), "Termian");
    }

    #[test]
    fn closing_active_tab_activates_left_neighbour() {
        let mut tabs = Tabs::default();
        tabs.open(session("Termian"));
        let launcher = tabs.open(TabKind::Launcher);
        tabs.close(launcher);
        assert_eq!(tabs.active().title(), "Termian");
        assert_eq!(tabs.all().len(), 3);
    }

    #[test]
    fn pinned_tabs_cannot_be_closed() {
        let mut tabs = Tabs::default();
        let pinned: Vec<_> = tabs.all().iter().map(|t| t.id).collect();
        for id in pinned {
            tabs.close(id);
        }
        assert_eq!(tabs.all().len(), 2);
    }

    #[test]
    fn closing_inactive_tab_keeps_active() {
        let mut tabs = Tabs::default();
        let a = tabs.open(session("a"));
        tabs.open(session("b"));
        tabs.close(a);
        assert_eq!(tabs.active().title(), "b");
    }
}
