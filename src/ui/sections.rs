//! Vaults sections other than Hosts: Keychain, Port Forwarding, Snippets,
//! Known Hosts, Logs.

use termian_native::language::text as tr;
use gpui::{div, prelude::*, px, rgb, svg, AnyElement, ClickEvent, Context, Div, SharedString, Stateful};
use termian_native::theme;

use super::pointer::PointerFeedback;
use super::assets::icon;
use super::hosts::{card, section_title, tile, tool_button};
use crate::Shell;

/// Toolbar row (left buttons, then the search, view and sort controls) plus
/// a scrollable body, shared by every section.
pub(super) fn page(id: &str, toolbar: Vec<Stateful<Div>>, controls: Vec<AnyElement>, body: impl IntoElement) -> Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(56.))
                .flex_none()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .border_b_1()
                .border_color(rgb(theme::TITLE_BAR))
                .bg(rgb(theme::SIDEBAR))
                .children(toolbar)
                .child(div().flex_1())
                .children(controls),
        )
        .child(div().id(SharedString::from(id.to_owned())).flex_1().overflow_y_scroll().px(px(BODY_PADDING)).pb_5().child(body))
}

/// A section without a toolbar (legacy Logs).
pub(super) fn bare_page(id: &str, body: impl IntoElement) -> Div {
    div().flex_1().h_full().flex().flex_col().child(div().id(SharedString::from(id.to_owned())).flex_1().overflow_y_scroll().px(px(BODY_PADDING)).pb_5().child(body))
}

/// Space between the sidebar and the cards, as in legacy.
pub(super) const BODY_PADDING: f32 = 30.;

/// Centered illustration and text shown when a section has no items.
pub(super) fn empty_state(icon_name: &str, title: &str, text: &str) -> Div {
    div()
        .pt(px(220.))
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .child(
            div()
                .size(px(60.))
                .rounded_xl()
                .bg(rgb(theme::SURFACE))
                .flex()
                .items_center()
                .justify_center()
                .child(svg().path(icon(icon_name)).size(px(28.)).text_color(rgb(theme::TEXT))),
        )
        .child(div().pt_4().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title.to_owned()))
        .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(text.to_owned()))
}

impl Shell {
    pub fn render_keychain(&self, window: &gpui::Window, cx: &mut Context<Self>) -> Div {
        let vault = &self.vault;
        let mut body = div();
        if !vault.keys.is_empty() {
            body = body.child(section_title(tr("Keys"))).child(self.cards("keychain").children(self.listed("keychain", &vault.keys, |k| k.label.as_str(), |k| k.id).into_iter().map(|key| {
                let id = key.id;
                super::keyboard::target(card(format!("key-{id}").into(), tile("keychain", theme::TILE_BLUE), key.label.clone(), key.subtitle()))
                    .on_click(cx.listener(move |shell, _, window, cx| shell.edit_key(id, window, cx)))
            })));
        }
        if !vault.identities.is_empty() {
            body = body.child(section_title(tr("Identities"))).child(self.cards("keychain").children(self.listed("keychain", &vault.identities, |i| i.label.as_str(), |i| i.id).into_iter().map(|identity| {
                let id = identity.id;
                super::keyboard::target(card(format!("identity-{id}").into(), tile("identity", theme::TILE_BLUE), identity.label.clone(), identity.subtitle().to_owned()))
                    .on_click(cx.listener(move |shell, _, window, cx| shell.edit_identity(id, window, cx)))
            })));
        }
        if vault.keys.is_empty() && vault.identities.is_empty() {
            body = body.child(empty_state("keychain", tr("Add a key"), tr("Keys and identities you save appear here.")));
        }
        let add_key = tool_button("new-key", "keychain", "KEY").rounded_r_none().on_click(cx.listener(|shell, _, window, cx| shell.new_key(window, cx)));
        // The split button's arrow, as in the legacy Keychain toolbar.
        let menu = div()
            .id("key-menu")
            .surface_feedback(theme::INPUT)
            .h(px(26.))
            .px_2()
            .flex()
            .items_center()
            .rounded_r_md()
            .bg(rgb(theme::INPUT))
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT)))
            .on_click(cx.listener(|shell, event: &ClickEvent, _, cx| {
                let at = event.position();
                shell.keychain_menu = Some(gpui::Point { x: at.x - px(20.), y: at.y + px(14.) });
                cx.notify();
            }));
        let split = div().id("key-split").flex().items_center().gap(px(1.)).child(add_key).child(menu);
        page("keychain-scroll", vec![split], self.listing_controls("keychain", true, false, window, cx), body)
    }


}
