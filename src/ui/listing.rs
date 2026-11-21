//! The right side of the legacy Vaults toolbar: search, «Grid / List» and
//! sorting menus, and on Hosts the tag filter; plus the New Forwarding
//! split-button menu.

use termian_native::language::text as tr;
use std::{cell::Cell, rc::Rc};

use gpui::{anchored, div, prelude::*, px, rgb, svg, AnyElement, Context, Corner, Div, FocusHandle, KeyDownEvent, MouseButton, Point, Pixels, SharedString, Stateful, Window};
use termian_native::listing::{self, Listing, SortOrder, ViewMode};
use termian_native::theme;
use termian_native::vault::RuleKind;

use super::assets::icon;
use super::dialog::Key;
use super::host_editor::menu_panel;
use super::pointer::PointerFeedback;
use super::text_input::{self, Mode};
use crate::Shell;

#[derive(Clone, Copy, PartialEq)]
pub enum MenuKind {
    View(&'static str),
    Sort(&'static str),
    Tags,
    NewForwarding,
}

/// Toolbar state shared by the Vaults sections.
#[derive(Default)]
pub struct ListingState {
    pub menu: Option<(MenuKind, Point<Pixels>)>,
    menu_invoker: Option<FocusHandle>,
    /// The search field opened by the magnifier, per section.
    pub query: Option<(&'static str, String)>,
    /// Hosts tag filter and the text typed in «Search tags».
    pub tags: Vec<String>,
    pub tag_query: String,
}

/// A small grey badge with letters, as legacy «Az», «L», «R», «D».
fn badge(text: &str) -> Div {
    div()
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.))
        .bg(rgb(theme::TEXT_MUTED))
        .text_color(rgb(theme::SURFACE))
        .text_size(px(8.))
        .font_weight(gpui::FontWeight::BOLD)
        .child(text.to_owned())
}

fn row(id: String, leading: impl IntoElement, label: &str, checked: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .h(px(40.))
        .px_3()
        .flex()
        .items_center()
        .gap_3()
        .rounded_md()
        .text_color(rgb(theme::TEXT))
        .menu_feedback(false)
        .child(leading)
        .child(div().flex_1().child(label.to_owned()))
        .when(checked, |d| d.child(svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::TEXT_MUTED))))
}

impl Shell {
    pub(crate) fn listing(&self, key: &str) -> Listing {
        self.vault.listing.get(key).copied().unwrap_or_default()
    }

    fn set_listing(&mut self, key: &str, change: impl FnOnce(&mut Listing), window: &mut Window, cx: &mut Context<Self>) {
        let mut listing = self.listing(key);
        change(&mut listing);
        self.vault.listing.insert(key.to_owned(), listing);
        self.save_vault();
        self.close_list_menu(window, cx);
    }

    /// Cards laid out as the section's view: as many columns as fit (legacy
    /// cards are at least about 220 px wide), or one.
    pub(crate) fn cards(&self, key: &str) -> Div {
        const MIN_CARD: f32 = 220.;
        const GAP: f32 = 12.;
        let max_columns = if key == "hosts" { 4 } else { 8 };
        let columns = match self.listing(key).view {
            ViewMode::Grid => (((self.cards_width.get() + GAP) / (MIN_CARD + GAP)).floor() as u16).clamp(1, max_columns),
            ViewMode::List => 1,
        };
        div().grid().grid_cols(columns).gap_3()
    }

    /// Items in the section's order, without those the search leaves out.
    pub(crate) fn listed<'a, T, K: Ord>(&self, key: &str, items: impl IntoIterator<Item = &'a T>, label: impl Fn(&T) -> &str, id: impl Fn(&T) -> K) -> Vec<&'a T>
    where
        T: 'a,
    {
        let query = self.list.query.as_ref().filter(|(k, _)| *k == key).map(|(_, q)| q.trim().to_lowercase()).unwrap_or_default();
        let mut items: Vec<&T> = items.into_iter().filter(|i| query.is_empty() || label(i).to_lowercase().contains(&query)).collect();
        self.listing(key).sort.sort(&mut items, |i| label(i), |i| id(i));
        items
    }

    fn open_list_menu(&mut self, kind: MenuKind, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.list.menu.is_some_and(|(k, _)| k == kind) {
            self.close_list_menu(window, cx);
            return;
        }
        self.list.menu_invoker = window.focused(cx);
        self.list.menu = Some((kind, at));
        window.focus(if kind == MenuKind::Tags { &self.tag_focus } else { &self.list_menu_focus });
        cx.notify();
    }

    fn close_list_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.menu = None;
        window.focus(self.list.menu_invoker.take().as_ref().unwrap_or(&self.root_focus));
        cx.notify();
    }

    fn list_menu_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let scope = &self.list_menu_focus;
        if super::keyboard::tab(event, window, cx, Some(scope)) {
            return;
        }
        let stroke = &event.keystroke;
        if stroke.modifiers.modified() {
            return;
        }
        match stroke.key.as_str() {
            "escape" => {
                self.close_list_menu(window, cx);
                cx.stop_propagation();
            }
            "down" => super::keyboard::step(window, cx, Some(scope), false),
            "up" => super::keyboard::step(window, cx, Some(scope), true),
            _ => {}
        }
    }

    pub(super) fn list_menu_button(
        &self,
        kind: MenuKind,
        leading: Option<Stateful<Div>>,
        button: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Div {
        let anchor = Rc::new(Cell::new(None));
        let click_anchor = anchor.clone();
        div()
            .flex()
            .items_center()
            .gap(px(1.))
            .children(leading)
            .child(button.on_click(cx.listener(move |shell, _, window, cx| {
                if let Some(at) = click_anchor.get() {
                    shell.open_list_menu(kind, at, window, cx);
                }
            })))
            .on_children_prepainted(move |bounds, _, _| {
                if let (Some(first), Some(last)) = (bounds.first(), bounds.last()) {
                    let x = if kind == MenuKind::NewForwarding { first.left() } else { last.right() };
                    anchor.set(Some(Point { x, y: last.bottom() + px(4.) }));
                }
            })
    }

    fn dropdown(&self, id: String, kind: MenuKind, leading: AnyElement, active: bool, cx: &mut Context<Self>) -> AnyElement {
        let open = self.list.menu.is_some_and(|(k, _)| k == kind);
        let color = if active { theme::ACCENT } else { theme::TEXT };
        let button = div()
            .id(SharedString::from(id))
            .h(px(28.))
            .px_1()
            .flex()
            .items_center()
            .gap(px(4.))
            .rounded_md()
            .when(open, |d| d.bg(rgb(theme::SURFACE_ACTIVE)).surface_feedback(theme::SURFACE_ACTIVE))
            .when(!open, |d| d.icon_feedback())
            .child(leading)
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(color)).when(open, |s| s.with_transformation(gpui::Transformation::rotate(gpui::radians(std::f32::consts::PI)))));
        self.list_menu_button(kind, None, button, cx).into_any_element()
    }

    /// Search, view and sort controls; `tags` adds the Hosts tag filter and
    /// `search` the magnifier (Hosts has its own search field).
    pub(crate) fn listing_controls(&self, key: &'static str, search: bool, tags: bool, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let listing = self.listing(key);
        let mut out = Vec::new();
        if search {
            out.push(match self.list.query.as_ref().filter(|(k, _)| *k == key) {
                Some((_, text)) => {
                    let focused = self.list_focus.is_focused(window);
                    let input = div()
                        .id("section-search")
                        .track_focus(&self.list_focus)
                        .on_key_down(cx.listener(Self::on_list_search_key))
                        .w(px(220.))
                        .h(px(30.))
                        .px_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .rounded_md()
                        .bg(rgb(theme::INPUT))
                        .border_1()
                        .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
                        .child(svg().path(icon("search")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                        .child(text_input::text("listing-search", text, tr("Search"), focused, Mode::Line, cx));
                    text_input::field(input, "listing-search", Mode::Line, |shell, window, _| window.focus(&shell.list_focus), cx)
                        .into_any_element()
                }
                None => div()
                    .id("section-search-open")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .icon_feedback()
                    .child(svg().path(icon("search")).size(px(18.)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        shell.list.query = Some((key, String::new()));
                        window.focus(&shell.list_focus);
                        cx.notify();
                    }))
                    .into_any_element(),
            });
        }
        let view_icon = match listing.view {
            ViewMode::Grid => "grid",
            ViewMode::List => "list",
        };
        out.push(self.dropdown(format!("{key}-view"), MenuKind::View(key), svg().path(icon(view_icon)).size(px(16.)).text_color(rgb(theme::TEXT)).into_any_element(), false, cx));
        if tags {
            let active = !self.list.tags.is_empty();
            out.push(self.dropdown("hosts-tags".into(), MenuKind::Tags, svg().path(icon("tag")).size(px(16.)).text_color(rgb(if active { theme::ACCENT } else { theme::TEXT })).into_any_element(), active, cx));
        }
        let sort_icon = match listing.sort {
            SortOrder::AZ => badge("Az").into_any_element(),
            SortOrder::ZA => badge("Za").into_any_element(),
            _ => svg().path(icon("calendar")).size(px(14.)).text_color(rgb(theme::TEXT)).into_any_element(),
        };
        out.push(self.dropdown(format!("{key}-sort"), MenuKind::Sort(key), sort_icon, false, cx));
        out
    }

    fn on_list_search_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some((_, text)) = self.list.query.as_mut() else { return };
        match text_input::edit("listing-search", Some(text), &event.keystroke, Mode::Line, cx) {
            Key::Escape => self.list.query = None,
            Key::Ignored => return,
            _ => {}
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn on_tag_search_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match text_input::edit("tag-search", Some(&mut self.list.tag_query), &event.keystroke, Mode::Line, cx) {
            Key::Escape => self.close_list_menu(window, cx),
            Key::Ignored => return,
            _ => {}
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The open toolbar menu, under the button that opened it.
    pub(crate) fn render_list_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (kind, at) = self.list.menu?;
        let width = match kind {
            MenuKind::Sort(_) => 240.,
            MenuKind::Tags => 300.,
            _ => 200.,
        };
        let mut panel = menu_panel("list-menu").w(px(width)).p_2().rounded(px(12.))
            .tab_group().track_focus(&self.list_menu_focus).on_key_down(cx.listener(Self::list_menu_key));
        match kind {
            MenuKind::View(key) => {
                let view = self.listing(key).view;
                for (mode, name, label) in [(ViewMode::Grid, "grid", tr("Grid")), (ViewMode::List, "list", tr("List"))] {
                    panel = panel.child(
                        row(format!("view-{label}"), svg().path(icon(name)).size(px(16.)).text_color(rgb(theme::TEXT_MUTED)), label, view == mode)
                            .on_click(cx.listener(move |shell, _, window, cx| shell.set_listing(key, |l| l.view = mode, window, cx))),
                    );
                }
            }
            MenuKind::Sort(key) => {
                let sort = self.listing(key).sort;
                for (i, order) in SortOrder::ALL.into_iter().enumerate() {
                    if i == 2 {
                        panel = panel.child(div().my_2().mx_2().h(px(1.)).bg(rgb(theme::SURFACE_ACTIVE)));
                    }
                    let leading = match order {
                        SortOrder::AZ => badge("Az").into_any_element(),
                        SortOrder::ZA => badge("Za").into_any_element(),
                        _ => svg().path(icon("calendar")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)).into_any_element(),
                    };
                    panel = panel.child(
                        row(format!("sort-{i}"), leading, order.label(), sort == order).on_click(cx.listener(move |shell, _, window, cx| shell.set_listing(key, |l| l.sort = order, window, cx))),
                    );
                }
            }
            MenuKind::Tags => {
                let focused = self.tag_focus.is_focused(window);
                let query = self.list.tag_query.to_lowercase();
                let input = div()
                        .id("tag-search")
                        .track_focus(&self.tag_focus)
                        .on_key_down(cx.listener(Self::on_tag_search_key))
                        .h(px(36.))
                        .px_3()
                        .mb_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .rounded_md()
                        .bg(rgb(theme::TITLE_BAR))
                        .border_1()
                        .border_color(rgb(if focused { theme::ACCENT } else { theme::SURFACE_ACTIVE }))
                        .child(svg().path(icon("search")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                        .child(text_input::text("tag-search", &self.list.tag_query, tr("Search tags"), focused, Mode::Line, cx));
                panel = panel.child(text_input::field(input, "tag-search", Mode::Line, |shell, window, _| window.focus(&shell.tag_focus), cx));
                for tag in listing::all_tags(&self.vault.hosts).into_iter().filter(|t| t.to_lowercase().contains(&query)) {
                    let chosen = self.list.tags.contains(&tag);
                    let circle = div()
                        .size(px(16.))
                        .rounded_full()
                        .border_2()
                        .border_color(rgb(if chosen { theme::ACCENT } else { theme::TEXT_MUTED }))
                        .when(chosen, |d| d.bg(rgb(theme::ACCENT)));
                    let name = tag.clone();
                    panel = panel.child(row(format!("tag-{tag}"), circle, &tag, false).on_click(cx.listener(move |shell, _, _, cx| {
                        match shell.list.tags.iter().position(|t| *t == name) {
                            Some(at) => {
                                shell.list.tags.remove(at);
                            }
                            None => shell.list.tags.push(name.clone()),
                        }
                        cx.notify();
                    })));
                }
                let any = !self.list.tags.is_empty();
                panel = panel.child(div().my_2().mx_2().h(px(1.)).bg(rgb(theme::SURFACE_ACTIVE))).child(
                    div()
                        .id("tags-clear")
                        .h(px(36.))
                        .px_3()
                        .flex()
                        .items_center()
                        .text_color(rgb(if any { theme::TEXT } else { theme::TEXT_MUTED }))
                        .when(any, |d| d.menu_feedback(false))
                        .child(tr("Clear selection"))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.list.tags.clear();
                            cx.notify();
                        })),
                );
            }
            MenuKind::NewForwarding => {
                for (kind, letter, label) in [(RuleKind::Local, "L", tr("Local Forwarding")), (RuleKind::Remote, "R", tr("Remote Forwarding")), (RuleKind::Dynamic, "D", tr("Dynamic Forwarding"))] {
                    panel = panel.child(row(format!("new-rule-{letter}"), badge(letter), label, false).on_click(cx.listener(move |shell, _, window, cx| {
                        shell.close_list_menu(window, cx);
                        shell.new_rule_of(kind, window, cx);
                    })));
                }
            }
        }
        let enter_id = match kind {
            MenuKind::View(_) => "view-menu-enter",
            MenuKind::Sort(_) => "sort-menu-enter",
            MenuKind::Tags => "tags-menu-enter",
            MenuKind::NewForwarding => "forwarding-menu-enter",
        };
        let menu = super::motion::popover(panel, enter_id);
        let corner = if kind == MenuKind::NewForwarding { Corner::TopLeft } else { Corner::TopRight };
        Some(
            div()
                .id("list-menu-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, window, cx| shell.close_list_menu(window, cx)))
                .child(anchored().position(at).anchor(corner).snap_to_window().child(menu))
                .into_any_element(),
        )
    }
}
