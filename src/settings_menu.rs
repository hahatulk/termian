#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppItem {
    Settings,
    Vaults,
    Sftp,
    FileAccess,
    Quit,
}

impl AppItem {
    pub const COMMON: [Self; 4] = [Self::Settings, Self::Vaults, Self::Sftp, Self::Quit];

    pub fn id(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Vaults => "vaults",
            Self::Sftp => "sftp",
            Self::FileAccess => "fileAccess",
            Self::Quit => "quit",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Settings => "Settings…",
            Self::Vaults => "Vaults",
            Self::Sftp => "SFTP",
            Self::FileAccess => "Доступ к файлам…",
            Self::Quit => "Quit Termian",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Account,
    Terminal,
    Sftp,
    Logs,
    Shortcuts,
    Version,
    Language,
}

impl Tab {
    pub const ALL: [Self; 7] = [
        Self::Account,
        Self::Terminal,
        Self::Sftp,
        Self::Logs,
        Self::Shortcuts,
        Self::Version,
        Self::Language,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Account => "Account",
            Self::Terminal => "Terminal",
            Self::Sftp => "SFTP",
            Self::Logs => "Logs",
            Self::Shortcuts => "Shortcuts",
            Self::Version => "Version",
            Self::Language => "Language",
        }
    }
}
