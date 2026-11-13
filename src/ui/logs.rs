//! Logs screen, as in the legacy client: one row per recorded session with
//! Date, User, Host and Saved columns; Date and Saved sort the list, the
//! bookmark marks a log as saved, and a click opens the recording read-only.

use gpui::{div, prelude::*, px, rgb, svg, Context, Div, FontWeight, SharedString};
use termian_native::session_log::{date_label, device_name, times_label};
use termian_native::theme;

use super::assets::icon;
use super::hosts::{os_tile, tile};
use super::sections::{empty_state, page};
use crate::Shell;

const DATE: f32 = 220.;
const USER: f32 = 240.;
const SAVED: f32 = 90.;

/// Sorting of the Logs list.
#[derive(Default)]
pub struct LogsView {
    oldest_first: bool,
    saved_first: bool,
}

fn header_cell(label: &str, width: Option<f32>) -> Div {
    div()
        .when_some(width, |d, w| d.w(px(w)).flex_none())
        .when(width.is_none(), |d| d.flex_1())
        .px(px(15.))
        .flex()
        .items_center()
        .gap(px(6.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(theme::TEXT))
        .child(label.to_owned())
}

impl Shell {
    pub fn render_logs(&self, cx: &mut Context<Self>) -> Div {
        let mut logs = self.vault.logs(self.logs_view.saved_first);
        if self.logs_view.oldest_first && !self.logs_view.saved_first {
            logs.reverse();
        }
        if logs.is_empty() {
            return page("logs-scroll", vec![], empty_state("logs", "Session logs", "Sessions you open are recorded here, encrypted. They are kept for 60 days."));
        }
        let arrow = |up: bool| svg().path(icon("chevron")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)).when(up, |s| s.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5))));
        let header = div()
            .h(px(40.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(theme::SURFACE_ACTIVE))
            .child(
                header_cell("Date", Some(DATE)).child(
                    div()
                        .id("logs-sort-date")
                        .px_1()
                        .rounded_sm()
                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                        .child(arrow(self.logs_view.oldest_first))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.logs_view.oldest_first = !shell.logs_view.oldest_first;
                            shell.logs_view.saved_first = false;
                            cx.notify();
                        })),
                ),
            )
            .child(header_cell("User", Some(USER)))
            .child(header_cell("Host", None))
            .child(
                header_cell("Saved", Some(SAVED)).child(
                    div()
                        .id("logs-sort-saved")
                        .px_1()
                        .rounded_sm()
                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                        .child(svg().path(icon(if self.logs_view.saved_first { "bookmark_filled" } else { "bookmark" })).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.logs_view.saved_first = !shell.logs_view.saved_first;
                            cx.notify();
                        })),
                ),
            );

        let device = device_name();
        let rows = logs.into_iter().map(|entry| {
            let id = entry.id;
            let host = entry.host.and_then(|h| self.vault.host(h));
            let icon_tile = match (host, entry.host) {
                (Some(host), _) => os_tile(host.os.as_deref()),
                (None, None) => tile("local", theme::TILE_BLUE),
                (None, Some(_)) => tile("new_host", theme::TILE_BLUE),
            };
            let times = entry.disconnected_at.map(|end| times_label(entry.connected_at, end)).unwrap_or_default();
            div()
                .id(SharedString::from(format!("log-{id}")))
                .min_h(px(60.))
                .flex()
                .items_start()
                .rounded_md()
                .hover(|s| s.bg(rgb(theme::SURFACE)))
                .child(
                    div()
                        .w(px(DATE))
                        .flex_none()
                        .px(px(15.))
                        .py(px(10.))
                        .child(div().text_color(rgb(theme::TEXT)).child(date_label(entry.connected_at)))
                        .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(times)),
                )
                // The native client has no account: the legacy "No account" cell with this device's name.
                .child(
                    div()
                        .w(px(USER))
                        .flex_none()
                        .px(px(15.))
                        .py(px(10.))
                        .flex()
                        .gap(px(10.))
                        .child(div().size(px(24.)).flex_none().rounded_full().bg(rgb(theme::SURFACE_ACTIVE)).flex().items_center().justify_center().child(svg().path(icon("identity")).size(px(12.)).text_color(rgb(theme::TEXT_MUTED))))
                        .child(div().min_w_0().child(div().text_color(rgb(theme::TEXT_MUTED)).child("No account")).child(div().text_sm().truncate().text_color(rgb(theme::TEXT_MUTED)).child(device.clone()))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px(px(15.))
                        .py(px(10.))
                        .flex()
                        .gap(px(10.))
                        .child(icon_tile.size(px(24.)).rounded(px(6.)))
                        .child(div().min_w_0().child(div().truncate().text_color(rgb(theme::TEXT)).child(entry.label.clone())).child(div().text_sm().truncate().text_color(rgb(theme::TEXT_MUTED)).child(entry.subtitle.clone()))),
                )
                .child(
                    div().w(px(SAVED)).flex_none().px(px(15.)).py(px(10.)).child(
                        div()
                            .id(SharedString::from(format!("log-saved-{id}")))
                            .size(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                            .child(svg().path(icon(if entry.saved { "bookmark_filled" } else { "bookmark" })).size(px(14.)).text_color(rgb(if entry.saved { theme::ACCENT } else { theme::TEXT_MUTED })))
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                cx.stop_propagation();
                                if let Some(entry) = shell.vault.history_entry_mut(id) {
                                    entry.saved = !entry.saved;
                                }
                                shell.save_vault();
                                cx.notify();
                            })),
                    ),
                )
                .on_click(cx.listener(move |shell, _, window, cx| shell.open_log(id, window, cx)))
        });
        page("logs-scroll", vec![], div().pt_3().child(header).children(rows))
    }
}
