//! New Group / Group Details: the legacy group slider (General: Label and
//! Parent Group), the group card's right-click menu, the "New Group" item
//! under NEW HOST, and "Remove a group" with the hosts that go with it.

use termian_native::language::text as tr;
use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Pixels, Point, Window};
use termian_native::theme;
use termian_native::vault::{Group, GroupId};

use super::pointer::PointerFeedback;
use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::host_editor::{card, field_box, menu_panel, menu_row};
use super::hosts::{host_tile, tile};
use super::text_input::{self, Mode};
use crate::Shell;

const WIDTH: f32 = 420.;

pub struct GroupEditor {
    group: Group,
    is_new: bool,
    created_here: bool,
    parents_open: bool,
    menu_open: bool,
    confirm_remove: bool,
}

/// Right-click menu of a group card, or the NEW HOST dropdown (`group: None`).
pub struct GroupMenu {
    group: Option<GroupId>,
    position: Point<Pixels>,
    confirm_remove: bool,
}

impl Shell {
    pub fn new_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        let group = Group { id: self.vault.next_id(), label: String::new(), parent: self.group };
        self.group_menu = None;
        self.host_editor = None;
        self.group_editor = Some(GroupEditor { group, is_new: true, created_here: true, parents_open: false, menu_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    pub fn edit_group(&mut self, id: GroupId, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        let Some(group) = self.vault.group(id).cloned() else { return };
        self.group_menu = None;
        self.host_editor = None;
        self.group_editor = Some(GroupEditor { group, is_new: false, created_here: false, parents_open: false, menu_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    /// Saved as typed once it has a label (the legacy form requires one).
    fn save_group_draft(&mut self) {
        let Some(editor) = &mut self.group_editor else { return };
        if editor.group.label.trim().is_empty() {
            return;
        }
        editor.is_new = false;
        self.vault.save_group(editor.group.clone());
        self.save_vault();
    }

    fn remove_group(&mut self, id: GroupId, cx: &mut Context<Self>) {
        self.vault.remove_group(id);
        self.save_vault();
        if self.group.is_some_and(|g| self.vault.group(g).is_none()) {
            self.group = None;
        }
        self.group_editor = None;
        self.group_menu = None;
        cx.notify();
    }

    fn on_group_editor_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.group_editor else { return };
        if editor.confirm_remove {
            if event.keystroke.key == "escape" {
                editor.confirm_remove = false;
                cx.notify();
            }
            return;
        }
        match text_input::edit("group-label", Some(&mut editor.group.label), &event.keystroke, Mode::Line, cx) {
            Key::Edited => self.save_group_draft(),
            Key::Enter => {}
            Key::Escape => self.group_editor = None,
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// "Remove a group": the group, then "And these hosts:" when it has any.
    fn remove_group_card(&self, id: GroupId, close: gpui::Stateful<Div>, cx: &mut Context<Self>) -> Div {
        let Some(group) = self.vault.group(id) else { return div() };
        let groups = self.vault.subtree(id);
        let hosts: Vec<_> = self.vault.hosts.iter().filter(|h| h.group.is_some_and(|g| groups.contains(&g))).collect();
        let row = |tile: Div, title: String, subtitle: String| {
            div()
                .flex()
                .items_center()
                .gap_3()
                .py(px(10.))
                .child(tile)
                .child(div().flex().flex_col().child(div().text_color(rgb(theme::TEXT)).child(title)).child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(subtitle)))
        };
        let count = self.vault.hosts_under(Some(id)).len();
        let mut body = div()
            .id("remove-group-list")
            .max_h(px(390.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .child(tr("You are going to remove this group:"))
            .child(row(tile("group", theme::TILE_BLUE), group.label.clone(), format!("{count} {}", if count == 1 { tr("Host") } else { tr("Hosts") })));
        if !hosts.is_empty() {
            body = body
                .child(div().pt(px(15.)).child(if hosts.len() == 1 { tr("And this host:") } else { tr("And these hosts:") }))
                .children(hosts.into_iter().map(|h| row(host_tile(h), h.display_label().to_owned(), h.subtitle())));
        }
        let title = match count {
            0 => tr("Remove a group").to_owned(),
            1 => tr("Remove a group and a host").to_owned(),
            n => format!("Remove a group and {n} hosts"),
        };
        dialog::card(&title, false, close, div().child(body), vec![dialog::button("remove-group-confirm", tr("Remove"), Kind::Error, false)
            .on_click(cx.listener(move |shell, _, _, cx| shell.remove_group(id, cx)))
            .into_any_element()])
    }

    pub fn render_group_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.group_editor.as_ref()?;
        let group = &editor.group;
        let focused = self.editor_focus.is_focused(window);
        let header = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(if editor.created_here { tr("New Group") } else { tr("Group Details") }))
            .when(!editor.is_new, |d| d.child(svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::TEXT))))
            .child(
                div()
                    .id("group-dots")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .child(svg().path(icon("dots")).w(px(16.)).h(px(4.)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.group_editor {
                            editor.menu_open = !editor.menu_open;
                        }
                        cx.notify();
                    })),
            )
            .child(dialog::collapse_button("group-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.group_editor = None;
                cx.notify();
            })));

        let label = field_box(focused).id("group-label").flex_1()
            .child(text_input::text("group-label", &group.label, tr("Label"), focused, Mode::Line, cx));
        let label = text_input::field(label, "group-label", Mode::Line, |shell, window, _| window.focus(&shell.editor_focus), cx);

        let parent = group.parent.map(|p| self.vault.group_path(p));
        let picker = field_box(editor.parents_open)
            .id("group-parent")
            .surface_feedback(theme::INPUT)
            .child(svg().path(icon("group")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(div().flex_1().text_color(rgb(if parent.is_some() { theme::TEXT } else { theme::TEXT_MUTED })).child(parent.unwrap_or_else(|| "Parent Group".into())))
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)))
            .on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.group_editor {
                    editor.parents_open = !editor.parents_open;
                }
                cx.notify();
            }));
        let parents = editor.parents_open.then(|| {
            let choice = |id: Option<GroupId>, label: String, cx: &mut Context<Self>| {
                menu_row(format!("group-parent-{id:?}"), &label, false, false).on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(editor) = &mut shell.group_editor {
                        editor.group.parent = id;
                        editor.parents_open = false;
                    }
                    shell.save_group_draft();
                    cx.notify();
                }))
            };
            let mut list = div().id("group-parent-list").max_h(px(220.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::INPUT)).shadow_lg().child(choice(None, tr("No group").into(), cx));
            for choice_group in self.vault.parent_choices(group.id) {
                list = list.child(choice(Some(choice_group.id), self.vault.group_path(choice_group.id), cx));
            }
            gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(list))
        });

        let general = card(tr("General"))
            .child(div().flex().items_center().gap_3().child(tile("group", theme::TILE_BLUE).size(px(36.)).rounded_lg()).child(label))
            .child(div().relative().child(picker).children(parents));

        let menu = editor.menu_open.then(|| {
            let disabled = editor.is_new;
            div().absolute().top(px(56.)).right(px(50.)).child(menu_panel("group-menu").child(menu_row("group-menu-remove".into(), tr("Remove"), true, disabled).when(!disabled, |d| {
                d.on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.group_editor {
                        editor.menu_open = false;
                        editor.confirm_remove = true;
                    }
                    cx.notify();
                }))
            })))
        });

        let confirm = editor.confirm_remove.then(|| {
            let close = dialog::close_button("remove-group-close").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.group_editor {
                    editor.confirm_remove = false;
                }
                cx.notify();
            }));
            dialog::backdrop(self.remove_group_card(group.id, close, cx))
        });

        let slider = div()
            .id("group-editor")
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_group_editor_key))
            .absolute()
            .top_0()
            .right(self.editor_inset())
            .bottom_0()
            .w(px(WIDTH))
            .occlude()
            .flex()
            .flex_col()
            .bg(rgb(theme::BACKGROUND))
            .border_l_1()
            .border_color(rgb(theme::TITLE_BAR))
            .shadow_lg()
            .child(header)
            .child(div().flex_1().p(px(15.)).flex().flex_col().gap(px(15.)).child(general))
            .children(menu);
        Some(div().absolute().inset_0().child(slider).children(confirm).into_any_element())
    }

    pub fn open_group_menu(&mut self, group: Option<GroupId>, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.group_menu = Some(GroupMenu { group, position, confirm_remove: false });
        cx.notify();
    }

    pub fn open_group_card_menu(&mut self, group: GroupId, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.open_group_menu(Some(group), event.position, cx);
    }

    /// Group card menu (Edit, Remove), or the NEW HOST dropdown (New Host, New Group).
    pub fn render_group_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.group_menu.as_ref()?;
        if let (true, Some(id)) = (menu.confirm_remove, menu.group) {
            let close = dialog::close_button("remove-group-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.group_menu = None;
                cx.notify();
            }));
            return Some(dialog::backdrop(self.remove_group_card(id, close, cx)).into_any_element());
        }
        let items = match menu.group {
            Some(id) => menu_panel("group-card-menu")
                .child(menu_row("group-menu-edit".into(), tr("Edit"), false, false).on_click(cx.listener(move |shell, _, window, cx| shell.edit_group(id, window, cx))))
                .child(menu_row("group-menu-remove".into(), tr("Remove"), true, false).on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(menu) = &mut shell.group_menu {
                        menu.confirm_remove = true;
                    }
                    cx.notify();
                }))),
            None => menu_panel("new-menu")
                .child(menu_row("new-menu-host".into(), tr("New Host"), false, false).on_click(cx.listener(|shell, _, window, cx| {
                    shell.group_menu = None;
                    shell.new_host(window, cx);
                })))
                .child(menu_row("new-menu-group".into(), tr("New Group"), false, false).on_click(cx.listener(|shell, _, window, cx| shell.new_group(window, cx)))),
        };
        let viewport = window.viewport_size();
        let left = menu.position.x.min(viewport.width - px(230.));
        let top = menu.position.y.min(viewport.height - px(90.));
        Some(
            div()
                .id("group-menu-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                    shell.group_menu = None;
                    cx.notify();
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(|shell, _, _, cx| {
                    shell.group_menu = None;
                    cx.notify();
                }))
                .child(div().absolute().left(left).top(top).child(super::motion::popover(items, "group-menu-enter")))
                .into_any_element(),
        )
    }
}
