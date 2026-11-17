//! What the connection screen shows while an SSH login is in progress:
//! the current stage, a log, and the question waiting for the user.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Connecting,
    /// Checking the server's host key.
    Verifying,
    Authenticating,
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKeyDecision {
    Reject,
    /// Trust for this connection only.
    Once,
    /// Trust and remember in known_hosts.
    Save,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostKeyInfo {
    pub host: String,
    pub port: u16,
    /// `ED25519`, `ECDSA`, `RSA`.
    pub algorithm: String,
    /// Base64 SHA-256 fingerprint without the `SHA256:` prefix.
    pub fingerprint: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoginEvent {
    Stage(Stage),
    Log(String),
    HostKey(HostKeyInfo),
    Prompt { text: String, echo: bool },
    /// The server's key differs from the saved one; the login is refused.
    HostKeyChanged(HostKeyInfo),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    HostKey(HostKeyDecision),
    /// `None` cancels the login.
    Text(Option<String>),
    /// An identity picked on the Username question: its username answers it
    /// and its password and key are used to authenticate.
    Identity { username: String, password: Option<String>, key: Option<crate::session::LoginKey> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Question {
    HostKey(HostKeyInfo),
    Text { prompt: String, echo: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoginState {
    pub stage: Stage,
    pub logs: Vec<String>,
    pub question: Option<Question>,
    /// What the user has typed for a text question.
    pub input: String,
    pub error: Option<String>,
    pub changed_key: Option<HostKeyInfo>,
    pub show_logs: bool,
    /// «Search identities» on the Username question: typed text, and
    /// whether keys go to it instead of the username.
    pub identity_query: String,
    pub searching_identities: bool,
    /// The «Continue & Save» dropdown with plain «Continue» is open.
    pub continue_menu: bool,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            stage: Stage::Connecting,
            logs: Vec::new(),
            question: None,
            input: String::new(),
            error: None,
            changed_key: None,
            show_logs: false,
            identity_query: String::new(),
            searching_identities: false,
            continue_menu: false,
        }
    }
}

impl LoginState {
    pub fn apply(&mut self, event: LoginEvent) {
        match event {
            LoginEvent::Stage(stage) => self.stage = stage,
            LoginEvent::Log(line) => self.logs.push(line),
            LoginEvent::HostKey(info) => {
                self.stage = Stage::Verifying;
                self.question = Some(Question::HostKey(info));
            }
            LoginEvent::Prompt { text, echo } => {
                self.question = Some(Question::Text { prompt: text, echo });
                self.input.clear();
            }
            LoginEvent::HostKeyChanged(info) => self.changed_key = Some(info),
        }
    }

    /// The login ended without a shell.
    pub fn fail(&mut self, reason: String) {
        self.question = None;
        self.error = Some(reason);
    }

    /// Answer for the pending question; clears it.
    pub fn answer(&mut self, answer: Answer) -> Option<Answer> {
        self.question.take()?;
        self.input.clear();
        self.identity_query.clear();
        self.searching_identities = false;
        self.continue_menu = false;
        Some(answer)
    }

    /// Submits the typed text for a text question.
    pub fn submit_text(&mut self) -> Option<Answer> {
        match self.question {
            Some(Question::Text { .. }) => {
                let text = std::mem::take(&mut self.input);
                self.answer(Answer::Text(Some(text)))
            }
            _ => None,
        }
    }

    /// The input as displayed: masked for passwords.
    pub fn shown_input(&self) -> String {
        match &self.question {
            Some(Question::Text { echo: false, .. }) => "•".repeat(self.input.chars().count()),
            _ => self.input.clone(),
        }
    }

    /// Progress of the three-step bar (connect, host key, terminal) as 0.0..=1.0.
    /// The Username question, which offers saved identities.
    pub fn asks_username(&self) -> bool {
        matches!(&self.question, Some(Question::Text { prompt, echo: true }) if prompt == "Username")
    }

    pub fn progress(&self) -> f32 {
        match self.stage {
            Stage::Connecting => 0.,
            Stage::Verifying | Stage::Authenticating => 0.5,
            Stage::Ready => 1.,
        }
    }
}

/// Clicks on the Username question that only change what is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginUi {
    TypeUsername,
    SearchIdentities,
    ToggleContinueMenu,
}

impl LoginState {
    pub fn apply_ui(&mut self, ui: LoginUi) {
        match ui {
            LoginUi::TypeUsername => self.searching_identities = false,
            LoginUi::SearchIdentities => self.searching_identities = true,
            LoginUi::ToggleContinueMenu => self.continue_menu = !self.continue_menu,
        }
    }
}

/// An identity offered on the Username question, as in the legacy
/// «Search identities» list.
#[derive(Clone, Debug, PartialEq)]
pub struct IdentityChoice {
    pub label: String,
    /// «Auth key», «Auth password», «Auth password and key».
    pub subtitle: &'static str,
    pub answer: Answer,
}

impl IdentityChoice {
    /// The vault's identities that have a username; their password and the
    /// Keychain key they use go with the answer.
    pub fn all(vault: &crate::vault::Vault) -> Vec<Self> {
        vault
            .identities
            .iter()
            .filter(|identity| !identity.username.is_empty())
            .map(|identity| {
                let key = identity.key.and_then(|id| vault.key(id)).and_then(crate::session::LoginKey::from_vault);
                Self {
                    label: identity.label.clone(),
                    subtitle: identity.subtitle(),
                    answer: Answer::Identity { username: identity.username.clone(), password: identity.password.clone(), key },
                }
            })
            .collect()
    }

    /// The «Search identities» filter: label or username, any case.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        let Answer::Identity { username, .. } = &self.answer else { return false };
        query.is_empty() || self.label.to_lowercase().contains(&query) || username.to_lowercase().contains(&query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> HostKeyInfo {
        HostKeyInfo { host: "192.168.1.180".into(), port: 22, algorithm: "ED25519".into(), fingerprint: "iF5O".into() }
    }

    #[test]
    fn host_key_question_moves_to_verifying_and_is_answered_once() {
        let mut state = LoginState::default();
        state.apply(LoginEvent::Log("Connecting".into()));
        state.apply(LoginEvent::HostKey(key()));
        assert_eq!(state.stage, Stage::Verifying);
        assert_eq!(state.progress(), 0.5);
        assert_eq!(state.answer(Answer::HostKey(HostKeyDecision::Save)), Some(Answer::HostKey(HostKeyDecision::Save)));
        assert_eq!(state.answer(Answer::HostKey(HostKeyDecision::Save)), None, "no question left");
        assert_eq!(state.logs, ["Connecting"]);
    }

    #[test]
    fn password_is_masked_and_submitted() {
        let mut state = LoginState::default();
        state.apply(LoginEvent::Prompt { text: "root@host's password:".into(), echo: false });
        state.input.push_str("s3cr");
        assert_eq!(state.shown_input(), "••••");
        assert_eq!(state.submit_text(), Some(Answer::Text(Some("s3cr".into()))));
        assert!(state.input.is_empty() && state.question.is_none());
    }

    #[test]
    fn submit_text_ignores_host_key_questions() {
        let mut state = LoginState::default();
        state.apply(LoginEvent::HostKey(key()));
        assert_eq!(state.submit_text(), None);
        assert!(state.question.is_some());
    }

    #[test]
    fn failure_clears_the_question() {
        let mut state = LoginState::default();
        state.apply(LoginEvent::Prompt { text: "Password:".into(), echo: false });
        state.fail("Permission denied".into());
        assert_eq!((state.question.clone(), state.error.as_deref()), (None, Some("Permission denied")));
    }

    #[test]
    fn identities_answer_the_username_with_their_credentials() {
        use crate::vault::{Identity, SshKey, Vault};
        let mut vault = Vault::default();
        vault.keys.push(SshKey { id: 7, label: "k".into(), private_key: Some("PRIVATE".into()), ..Default::default() });
        vault.identities.push(Identity { id: 1, label: "root".into(), has_key: true, username: "root".into(), key: Some(7), ..Default::default() });
        vault.identities.push(Identity { id: 2, label: "nameless".into(), has_password: true, ..Default::default() });
        let choices = IdentityChoice::all(&vault);
        assert_eq!(choices.len(), 1, "an identity without a username cannot answer");
        assert_eq!(choices[0].subtitle, "Auth key");
        assert_eq!(choices[0].answer, Answer::Identity { username: "root".into(), password: None, key: Some(crate::session::LoginKey::new("PRIVATE", None)) });
        assert!(choices[0].matches("RO") && !choices[0].matches("pt"));
    }
}
