//! The legacy Port Forwarding Wizard: a new rule filled in one question at a
//! time. The steps depend on the rule kind; Continue is enabled once the
//! step's required fields are set, and the last step (Label) saves the rule.

use crate::vault::{ForwardingRule, RuleKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Kind,
    LocalPort,
    RemotePort,
    Host,
    Destination,
    Label,
}

pub fn steps(kind: RuleKind) -> &'static [Step] {
    match kind {
        RuleKind::Local => &[Step::Kind, Step::LocalPort, Step::Host, Step::Destination, Step::Label],
        RuleKind::Remote => &[Step::Kind, Step::Host, Step::RemotePort, Step::Destination, Step::Label],
        RuleKind::Dynamic => &[Step::Kind, Step::LocalPort, Step::Host, Step::Label],
    }
}

/// The step after `step`, or `None` when `step` is the last one (Done).
pub fn next(kind: RuleKind, step: Step) -> Option<Step> {
    let steps = steps(kind);
    steps.iter().position(|s| *s == step).and_then(|at| steps.get(at + 1)).copied()
}

/// The step before `step`; the first step stays where it is.
pub fn back(kind: RuleKind, step: Step) -> Step {
    let steps = steps(kind);
    match steps.iter().position(|s| *s == step) {
        Some(at) if at > 0 => steps[at - 1],
        _ => steps[0],
    }
}

/// Whether Continue (or Done) is enabled on `step`.
pub fn can_continue(step: Step, rule: &ForwardingRule) -> bool {
    match step {
        Step::Kind | Step::Label => true,
        Step::LocalPort | Step::RemotePort => rule.port.is_some(),
        Step::Host => rule.host.is_some(),
        Step::Destination => !rule.destination.trim().is_empty() && rule.destination_port.is_some(),
    }
}

/// The slider title: "New Port Forwarding" while choosing the kind.
pub fn title(step: Step, kind: RuleKind) -> &'static str {
    match (step, kind) {
        (Step::Kind, _) => "New Port Forwarding",
        (_, RuleKind::Local) => "Local Port Forwarding",
        (_, RuleKind::Remote) => "Remote Port Forwarding",
        (_, RuleKind::Dynamic) => "Dynamic Port Forwarding",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_follow_the_legacy_order() {
        assert_eq!(steps(RuleKind::Local), [Step::Kind, Step::LocalPort, Step::Host, Step::Destination, Step::Label]);
        assert_eq!(steps(RuleKind::Remote), [Step::Kind, Step::Host, Step::RemotePort, Step::Destination, Step::Label]);
        assert_eq!(steps(RuleKind::Dynamic), [Step::Kind, Step::LocalPort, Step::Host, Step::Label]);
    }

    #[test]
    fn next_and_back_walk_the_steps() {
        assert_eq!(next(RuleKind::Dynamic, Step::LocalPort), Some(Step::Host));
        assert_eq!(next(RuleKind::Dynamic, Step::Host), Some(Step::Label));
        assert_eq!(next(RuleKind::Dynamic, Step::Label), None);
        assert_eq!(back(RuleKind::Remote, Step::RemotePort), Step::Host);
        assert_eq!(back(RuleKind::Remote, Step::Kind), Step::Kind);
    }

    #[test]
    fn continue_needs_the_step_fields() {
        let mut rule = ForwardingRule::new(1, RuleKind::Local);
        assert!(can_continue(Step::Kind, &rule));
        assert!(!can_continue(Step::LocalPort, &rule));
        rule.port = Some(8080);
        assert!(can_continue(Step::LocalPort, &rule));
        assert!(!can_continue(Step::Host, &rule));
        rule.destination = " ".into();
        rule.destination_port = Some(80);
        assert!(!can_continue(Step::Destination, &rule));
        rule.destination = "10.0.0.5".into();
        assert!(can_continue(Step::Destination, &rule));
        assert!(can_continue(Step::Label, &rule));
    }

    #[test]
    fn title_names_the_kind_after_the_first_step() {
        assert_eq!(title(Step::Kind, RuleKind::Remote), "New Port Forwarding");
        assert_eq!(title(Step::Host, RuleKind::Remote), "Remote Port Forwarding");
    }
}
