//! Snippets screen as in legacy: Packages and Snippets, «All snippets ›
//! package», the New Snippet / Edit Snippet slider (Action description,
//! Package, Script, Targets), New Package / Edit Package, the card menu
//! (Run, Edit, Duplicate, Remove) and «Add targets» / «Select targets».
//! «Run» opens an SSH tab per target and types the script once it is ready.

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, ClickEvent, Context, Div, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Pixels, Point, SharedString, Stateful, Window};
use termian_native::snippets::{self, Coverage, PackageChoice, LABEL_LIMIT, MAX_TARGETS};
use termian_native::theme;
use termian_native::vault::{GroupId, HostId, Snippet, SnippetPackage};

use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::host_editor::{caret, field_box, menu_panel, menu_row};
use super::hosts::{card, host_tile, section_title, tile, tool_button};
use super::sections::{empty_state, page};
use crate::Shell;

const WIDTH: f32 = 420.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Label,
    Package,
    Script,
    Search,
}

/// The slider's screens: the form, «Add targets» (saved with the snippet)
/// and «Select targets» (before a run).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Edit,
    AddTargets,
    SelectTargets,
}

pub struct SnippetEditor {
    snippet: Snippet,
    created_here: bool,
    field: Field,
    /// Typed into the Package field.
    package_query: String,
    packages_open: bool,
    /// A package to create when the snippet is saved.
    new_package: Option<String>,
    route: Route,
    picked: Vec<HostId>,
    picker_group: Option<GroupId>,
    search: String,
    menu_open: bool,
    confirm_remove: bool,
}

/// The Shell History slider: rows expand on click; SAVE asks for a label
/// and makes a snippet.
#[derive(Default)]
pub struct ShellHistoryPanel {
    expanded: Option<usize>,
    editing: Option<(usize, String)>,
    saved: Vec<usize>,
    menu_open: bool,
}

pub struct PackageEditor {
    package: SnippetPackage,
    created_here: bool,
    menu_open: bool,
    confirm_remove: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Snippet(u64),
    Package(u64),
    /// The NEW SNIPPET split button's arrow.
    New,
}

pub struct SnippetMenu {
    item: MenuItem,
    position: Point<Pixels>,
    confirm_remove: bool,
}

fn editor(snippet: Snippet, created_here: bool, route: Route) -> SnippetEditor {
    let picked = if route == Route::AddTargets { snippet.targets.clone() } else { Vec::new() };
    SnippetEditor {
        snippet,
        created_here,
        field: Field::Label,
        package_query: String::new(),
        packages_open: false,
        new_package: None,
        route,
        picked,
        picker_group: None,
        search: String::new(),
        menu_open: false,
        confirm_remove: false,
    }
}

fn header(title: &str) -> Div {
    div()
        .h(px(64.))
        .flex_none()
        .px(px(15.))
        .flex()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(rgb(theme::TITLE_BAR))
        .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title.to_owned()))
}

fn dots(id: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
        .child(svg().path(icon("dots")).w(px(16.)).h(px(4.)).text_color(rgb(theme::TEXT)))
}

fn slider(id: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(px(WIDTH))
        .occlude()
        .flex()
        .flex_col()
        .bg(rgb(theme::BACKGROUND))
        .border_l_1()
        .border_color(rgb(theme::TITLE_BAR))
        .shadow_lg()
}

/// Text with a caret at the end, or the placeholder when empty.
fn input_text(value: &str, placeholder: &str, focused: bool) -> Div {
    let empty = value.is_empty();
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .overflow_hidden()
        .text_color(rgb(if empty { theme::TEXT_MUTED } else { theme::TEXT }))
        .when(focused && empty, |d| d.child(caret()))
        .child(div().truncate().child(if empty { placeholder.to_owned() } else { value.to_owned() }))
        .when(focused && !empty, |d| d.child(caret()))
}

fn checkbox(state: Coverage) -> Div {
    let on = state != Coverage::None;
    div()
        .size(px(16.))
        .flex_none()
        .rounded_sm()
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(rgb(if on { theme::ACCENT } else { theme::TEXT_MUTED }))
        .when(on, |d| d.bg(rgb(theme::ACCENT)))
        .map(|d| match state {
            Coverage::All => d.child(svg().path(icon("check")).size(px(10.)).text_color(rgb(0xffffff))),
            Coverage::Some => d.child(div().w(px(8.)).h(px(2.)).bg(rgb(0xffffff))),
            Coverage::None => d,
        })
}

fn host_row(id: String, tile: Div, title: String, subtitle: String) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .h(px(60.))
        .px(px(10.))
        .flex()
        .items_center()
        .gap(px(10.))
        .rounded_lg()
        .child(tile)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().text_color(rgb(theme::TEXT)).child(title))
                .child(div().truncate().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(subtitle)),
        )
}

impl Shell {
    // ── Screen ──────────────────────────────────────────────────────────

    pub fn render_snippets(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let vault = &self.vault;
        let open = self.snippet_package.and_then(|p| vault.package(p));
        let snippets = vault.snippets_in(open.map(|p| p.id), false);
        let mut body = div();
        if let Some(package) = open {
            let crumb = div()
                .id("snippets-crumb-all")
                .text_color(rgb(theme::ACCENT))
                .child("All snippets")
                .on_click(cx.listener(|shell, _, _, cx| {
                    shell.snippet_package = None;
                    shell.close_snippet_sliders();
                    cx.notify();
                }));
            body = body.child(
                div().pt_5().flex().items_center().gap_2().text_sm().child(crumb).child(div().text_color(rgb(theme::TEXT_MUTED)).child("›")).child(div().text_color(rgb(theme::TEXT)).child(package.label.clone())),
            );
        } else if !vault.packages.is_empty() {
            body = body.child(section_title("Packages")).child(self.cards("snippets").children(self.listed("snippets", &vault.packages, |p| p.label.as_str(), |p| p.id).into_iter().map(|p| self.render_package_card(p, cx))));
        }
        if !snippets.is_empty() {
            let snippets = self.listed("snippets", snippets, |s| if s.label.is_empty() { s.script.as_str() } else { s.label.as_str() }, |s| s.id);
            body = body.child(section_title("Snippets")).child(self.cards("snippets").children(snippets.into_iter().map(|s| self.render_snippet_card(s, cx))));
        } else if open.is_some() {
            body = body.child(empty_state("snippets_package", "This package is empty", "Snippets you add to it appear here."));
        } else if vault.packages.is_empty() {
            body = body.child(empty_state("snippets", "Create a snippet", "Save commands you run often."));
        }

        let add = tool_button("new-snippet", "snippets", "NEW SNIPPET").rounded_r_none().on_click(cx.listener(|shell, _, window, cx| shell.new_snippet(window, cx)));
        let arrow = div()
            .id("new-snippet-menu")
            .h(px(26.))
            .px_2()
            .flex()
            .items_center()
            .rounded_r_md()
            .bg(rgb(theme::INPUT))
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT)))
            .on_click(cx.listener(|shell, event: &ClickEvent, _, cx| {
                let at = event.position();
                shell.snippet_menu = Some(SnippetMenu { item: MenuItem::New, position: Point { x: at.x - px(20.), y: at.y + px(14.) }, confirm_remove: false });
                cx.notify();
            }));
        let split = div().id("new-snippet-split").flex().items_center().gap(px(1.)).child(add).child(arrow);
        let history = tool_button("shell-history", "logs", "SHELL HISTORY").on_click(cx.listener(|shell, _, window, cx| {
            shell.close_snippet_sliders();
            shell.shell_history = Some(ShellHistoryPanel::default());
            window.focus(&shell.editor_focus);
            cx.notify();
        }));
        page("snippets-scroll", vec![split, history], self.listing_controls("snippets", true, false, window, cx), body)
    }

    fn edit_pencil(id: String, hover: SharedString) -> Stateful<Div> {
        div()
            .id(SharedString::from(id))
            .size(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .invisible()
            .group_hover(hover, |s| s.visible())
            .hover(|s| s.bg(rgb(theme::INPUT)))
            .child(svg().path(icon("edit")).size(px(12.)).text_color(rgb(theme::TEXT)))
    }

    fn render_package_card(&self, package: &SnippetPackage, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = package.id;
        let count = self.vault.package_snippet_count(id);
        let hover = SharedString::from(format!("package-card-{id}"));
        let edit = Self::edit_pencil(format!("package-edit-{id}"), hover.clone()).on_click(cx.listener(move |shell, _, window, cx| {
            cx.stop_propagation();
            shell.edit_package(id, window, cx);
        }));
        card(format!("package-{id}").into(), tile("snippets_package", theme::TILE_BLUE), package.label.clone(), format!("{count} {}", if count == 1 { "snippet" } else { "snippets" }))
            .group(hover)
            .child(edit)
            .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                if event.click_count() >= 2 {
                    shell.close_snippet_sliders();
                    shell.snippet_package = Some(id);
                    cx.notify();
                }
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                shell.snippet_menu = Some(SnippetMenu { item: MenuItem::Package(id), position: event.position, confirm_remove: false });
                cx.notify();
            }))
    }

    fn render_snippet_card(&self, snippet: &Snippet, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = snippet.id;
        let hover = SharedString::from(format!("snippet-card-{id}"));
        let edit = Self::edit_pencil(format!("snippet-edit-{id}"), hover.clone()).on_click(cx.listener(move |shell, _, window, cx| {
            cx.stop_propagation();
            shell.edit_snippet(id, window, cx);
        }));
        card(format!("snippet-{id}").into(), tile("snippets", theme::TILE_BLUE), snippet.display_label().to_owned(), snippet.subtitle().unwrap_or_default().to_owned())
            .group(hover)
            .child(edit)
            .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    shell.execute_snippet(id, window, cx);
                }
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                shell.snippet_menu = Some(SnippetMenu { item: MenuItem::Snippet(id), position: event.position, confirm_remove: false });
                cx.notify();
            }))
    }

    fn close_snippet_sliders(&mut self) {
        self.shell_history = None;
        self.snippet_editor = None;
        self.package_editor = None;
        self.snippet_menu = None;
        self.shell_history = None;
    }

    // ── Run ─────────────────────────────────────────────────────────────

    /// Legacy `executeSnippetOnHosts`: saved targets run at once, otherwise
    /// «Select targets» opens.
    pub fn execute_snippet(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snippet) = self.vault.snippet(id).cloned() else { return };
        self.snippet_menu = None;
        let targets: Vec<HostId> = snippet.targets.iter().copied().filter(|h| self.vault.host(*h).is_some()).collect();
        if targets.is_empty() {
            self.package_editor = None;
            self.snippet_editor = Some(editor(snippet, false, Route::SelectTargets));
            if let Some(editor) = &mut self.snippet_editor {
                editor.field = Field::Search;
            }
            window.focus(&self.editor_focus);
            cx.notify();
        } else {
            self.run_snippet(snippet.display_label(), &snippet.script, &targets, window, cx);
        }
    }

    /// An SSH tab per target; each types the script once its shell is ready.
    /// One target opens a tab; several open one Split View workspace, as legacy does.
    fn run_snippet(&mut self, title: &str, script: &str, targets: &[HostId], window: &mut Window, cx: &mut Context<Self>) {
        self.close_snippet_sliders();
        match targets {
            [host] => self.connect_host_with(*host, Some(script.to_owned()), window, cx),
            hosts => self.open_workspace_for(title, hosts, script, window, cx),
        }
        cx.notify();
    }

    // ── Snippet slider ──────────────────────────────────────────────────

    pub fn new_snippet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snippet = Snippet { id: self.vault.next_id(), package: self.snippet_package, ..Snippet::default() };
        self.package_editor = None;
        self.snippet_menu = None;
        self.shell_history = None;
        self.snippet_editor = Some(editor(snippet, true, Route::Edit));
        window.focus(&self.editor_focus);
        cx.notify();
    }

    pub fn edit_snippet(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snippet) = self.vault.snippet(id).cloned() else { return };
        self.package_editor = None;
        self.snippet_menu = None;
        self.shell_history = None;
        self.snippet_editor = Some(editor(snippet, false, Route::Edit));
        window.focus(&self.editor_focus);
        cx.notify();
    }

    /// Saved as typed once it has a script (the legacy form requires one).
    fn save_snippet_draft(&mut self) {
        let Some(editor) = &mut self.snippet_editor else { return };
        if editor.snippet.script.trim().is_empty() {
            return;
        }
        if let Some(name) = editor.new_package.take() {
            editor.snippet.package = Some(self.vault.package_named(&name));
        }
        self.vault.save_snippet(editor.snippet.clone());
        self.save_vault();
    }

    fn choose_package(&mut self, choice: Option<PackageChoice>) {
        let Some(editor) = &mut self.snippet_editor else { return };
        match choice {
            Some(PackageChoice::Existing(id, _)) => {
                editor.snippet.package = Some(id);
                editor.new_package = None;
            }
            Some(PackageChoice::Create(name)) => {
                editor.snippet.package = None;
                editor.new_package = Some(name);
            }
            None => {
                editor.snippet.package = None;
                editor.new_package = None;
            }
        }
        editor.package_query.clear();
        editor.packages_open = false;
        editor.field = Field::Script;
        self.save_snippet_draft();
    }

    fn on_snippet_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(editor) = &mut self.snippet_editor else { return };
        if editor.confirm_remove {
            if keystroke.key == "escape" {
                editor.confirm_remove = false;
                cx.notify();
            }
            return;
        }
        if keystroke.key == "escape" {
            match editor.route {
                Route::AddTargets => editor.route = Route::Edit,
                Route::Edit | Route::SelectTargets => self.snippet_editor = None,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if keystroke.key == "tab" && editor.route == Route::Edit {
            editor.field = match editor.field {
                Field::Label => Field::Package,
                Field::Package => Field::Script,
                Field::Script | Field::Search => Field::Label,
            };
            editor.packages_open = editor.field == Field::Package;
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if keystroke.key == "enter" && editor.field == Field::Package {
            let first = snippets::package_choices(&self.vault, &editor.package_query).into_iter().next();
            if first.is_some() {
                self.choose_package(first);
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let field = editor.field;
        let value = match field {
            Field::Label => &mut editor.snippet.label,
            Field::Package => &mut editor.package_query,
            Field::Script => &mut editor.snippet.script,
            Field::Search => &mut editor.search,
        };
        let edited = if field == Field::Script {
            // The script keeps its lines; Enter starts a new one.
            match keystroke.key.as_str() {
                _ if keystroke.modifiers.platform && keystroke.key == "v" => {
                    value.push_str(&paste.unwrap_or_default());
                    true
                }
                _ if keystroke.modifiers.platform || keystroke.modifiers.control => false,
                "backspace" => {
                    value.pop();
                    true
                }
                "enter" => {
                    value.push('\n');
                    true
                }
                _ => match keystroke.key_char.as_deref() {
                    Some(text) if !text.chars().any(char::is_control) => {
                        value.push_str(text);
                        true
                    }
                    _ => false,
                },
            }
        } else {
            matches!(dialog::edit(Some(value), keystroke, paste), Key::Edited)
        };
        if !edited {
            return;
        }
        match field {
            Field::Label => {
                if editor.snippet.label.chars().count() > LABEL_LIMIT {
                    editor.snippet.label = editor.snippet.label.chars().take(LABEL_LIMIT).collect();
                }
                self.save_snippet_draft();
            }
            Field::Script => self.save_snippet_draft(),
            Field::Package => editor.packages_open = true,
            Field::Search => {}
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn focus_snippet_field(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.snippet_editor {
            editor.field = field;
            editor.packages_open = field == Field::Package;
            editor.menu_open = false;
        }
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn snippet_menu_rows(&self, id: u64, saved: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let disabled = !saved;
        let panel = menu_panel("snippet-form-menu")
            .child(menu_row("snippet-form-run".into(), "Run", false, disabled).when(saved, |d| d.on_click(cx.listener(move |shell, _, window, cx| shell.execute_snippet(id, window, cx)))))
            .child(menu_row("snippet-form-duplicate".into(), "Duplicate", false, disabled).when(saved, |d| {
                d.on_click(cx.listener(move |shell, _, window, cx| {
                    if let Some(copy) = shell.vault.duplicate_snippet(id) {
                        shell.save_vault();
                        shell.edit_snippet(copy, window, cx);
                    }
                }))
            }))
            .child(menu_row("snippet-form-remove".into(), "Remove", true, disabled).when(saved, |d| {
                d.on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.snippet_editor {
                        editor.menu_open = false;
                        editor.confirm_remove = true;
                    }
                    cx.notify();
                }))
            }));
        panel
    }

    pub fn render_snippet_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.snippet_editor.as_ref()?;
        let focused = self.editor_focus.is_focused(window);
        let body = match editor.route {
            Route::Edit => self.render_snippet_form(editor, focused, cx),
            Route::AddTargets | Route::SelectTargets => self.render_target_picker(editor, focused, cx),
        };
        let id = editor.snippet.id;
        let confirm = editor.confirm_remove.then(|| {
            let close = dialog::close_button("snippet-remove-close").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.snippet_editor {
                    editor.confirm_remove = false;
                }
                cx.notify();
            }));
            self.remove_card(&[], &[id], close, cx)
        });
        let slider = slider("snippet-editor").track_focus(&self.editor_focus).on_key_down(cx.listener(Self::on_snippet_key)).child(body);
        Some(match confirm {
            Some(confirm) => div().absolute().inset_0().child(slider).child(confirm).into_any_element(),
            None => slider.into_any_element(),
        })
    }

    fn render_snippet_form(&self, editor: &SnippetEditor, focused: bool, cx: &mut Context<Self>) -> Div {
        let snippet = &editor.snippet;
        let saved = self.vault.snippet(snippet.id).is_some();
        let on = |field: Field| focused && editor.field == field;
        let head = header(if editor.created_here && !saved { "New Snippet" } else { "Edit Snippet" })
            .when(saved, |d| d.child(svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::TEXT))))
            .child(dots("snippet-dots").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.snippet_editor {
                    editor.menu_open = !editor.menu_open;
                    editor.packages_open = false;
                }
                cx.notify();
            })))
            .child(dialog::close_button("snippet-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.snippet_editor = None;
                cx.notify();
            })));

        let label = field_box(on(Field::Label))
            .id("snippet-label")
            .flex_1()
            .child(input_text(&snippet.label, "Action description", on(Field::Label)))
            .on_click(cx.listener(|shell, _, window, cx| shell.focus_snippet_field(Field::Label, window, cx)));

        // Package: a typed name filters the list or creates a package.
        let chosen = editor.new_package.clone().or_else(|| snippet.package.and_then(|p| self.vault.package(p)).map(|p| p.label.clone()));
        let package_text = if on(Field::Package) && (editor.packages_open || chosen.is_none()) { editor.package_query.clone() } else { chosen.clone().unwrap_or_default() };
        let package = field_box(on(Field::Package))
            .id("snippet-package")
            .child(svg().path(icon("snippets_package")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(input_text(&package_text, if chosen.is_some() && on(Field::Package) { chosen.as_deref().unwrap_or_default() } else { "Add a Package" }, on(Field::Package)))
            .when(chosen.is_some(), |d| {
                d.child(
                    div()
                        .id("snippet-package-remove")
                        .px_2()
                        .text_sm()
                        .text_color(rgb(theme::ACCENT))
                        .child("Remove")
                        .on_click(cx.listener(|shell, _, _, cx| {
                            cx.stop_propagation();
                            shell.choose_package(None);
                            cx.notify();
                        })),
                )
            })
            .on_click(cx.listener(|shell, _, window, cx| shell.focus_snippet_field(Field::Package, window, cx)));
        let packages = (editor.packages_open && on(Field::Package)).then(|| {
            let choices = snippets::package_choices(&self.vault, &editor.package_query);
            let mut list = div().id("snippet-package-list").max_h(px(220.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::SURFACE)).border_1().border_color(rgb(theme::SURFACE_ACTIVE)).shadow_lg();
            if choices.is_empty() {
                list = list.child(div().px_3().py_2().text_sm().text_color(rgb(theme::TEXT_MUTED)).child("Create a new package by typing the name"));
            }
            for (i, choice) in choices.into_iter().enumerate() {
                let text = match &choice {
                    PackageChoice::Existing(_, label) => label.clone(),
                    PackageChoice::Create(name) => format!("Create \"{name}\""),
                };
                list = list.child(menu_row(format!("snippet-package-{i}"), &text, false, false).on_click(cx.listener(move |shell, _, _, cx| {
                    shell.choose_package(Some(choice.clone()));
                    cx.notify();
                })));
            }
            gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(list))
        });

        let lines: Vec<String> = if snippet.script.is_empty() { vec![String::new()] } else { snippet.script.split('\n').map(str::to_owned).collect() };
        let last = lines.len() - 1;
        let script_on = on(Field::Script);
        let script = div()
            .id("snippet-script")
            .min_h(px(132.))
            .max_h(px(220.))
            .overflow_y_scroll()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .border_1()
            .border_color(rgb(if script_on { theme::ACCENT } else { theme::INPUT }))
            .font_family("Menlo")
            .text_sm()
            .text_color(rgb(if snippet.script.is_empty() { theme::TEXT_MUTED } else { theme::TEXT }))
            .children(lines.into_iter().enumerate().map(move |(i, line)| {
                let placeholder = i == 0 && line.is_empty() && last == 0;
                div()
                    .flex()
                    .items_center()
                    .min_h(px(18.))
                    .when(placeholder && script_on, |d| d.child(caret()))
                    .child(if placeholder { "Script *".to_owned() } else { line })
                    .when(script_on && i == last && !placeholder, |d| d.child(caret()))
            }))
            .on_click(cx.listener(|shell, _, window, cx| shell.focus_snippet_field(Field::Script, window, cx)));

        let form = div()
            .p(px(15.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .child(div().flex().items_center().gap_3().child(tile("snippets", theme::TILE_BLUE).size(px(36.)).rounded_lg()).child(label))
            .child(div().relative().child(package).children(packages))
            .child(script);

        let menu = editor.menu_open.then(|| div().absolute().top(px(56.)).right(px(50.)).child(self.snippet_menu_rows(snippet.id, saved, cx)));
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .child(head)
            .child(div().id("snippet-form-scroll").flex_1().overflow_y_scroll().p(px(15.)).flex().flex_col().gap(px(15.)).child(form).child(self.render_targets_card(snippet, saved, cx)))
            .children(menu)
    }

    /// «Targets for execution» with «Add targets», or the saved targets with «Edit».
    fn render_targets_card(&self, snippet: &Snippet, saved: bool, cx: &mut Context<Self>) -> Div {
        let open_picker = move |shell: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
            if let Some(editor) = &mut shell.snippet_editor {
                editor.picked = editor.snippet.targets.clone();
                editor.route = Route::AddTargets;
                editor.field = Field::Search;
                editor.search.clear();
                editor.picker_group = None;
                editor.menu_open = false;
                editor.packages_open = false;
            }
            window.focus(&shell.editor_focus);
            cx.notify();
        };
        let card = div().p(px(15.)).flex().flex_col().gap(px(10.)).rounded_lg().bg(rgb(theme::SURFACE));
        let targets: Vec<_> = snippet.targets.iter().filter_map(|h| self.vault.host(*h)).collect();
        if targets.is_empty() {
            let placeholder = || host_row(String::new(), tile("new_host", theme::SURFACE_ACTIVE), "IP or Hostname".into(), "SSH".into()).bg(rgb(theme::BACKGROUND)).opacity(0.6);
            return card
                .child(div().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child("Targets for execution"))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Automate work with snippets. Add target hosts and automatically run the snippet on them in one click!"))
                .child(placeholder().id("target-placeholder-1"))
                .child(placeholder().id("target-placeholder-2"))
                .child(dialog::button("snippet-add-targets", "Add targets", Kind::Text, !saved).text_color(rgb(theme::ACCENT)).when(saved, |d| d.on_click(cx.listener(move |shell, _, window, cx| open_picker(shell, window, cx)))));
        }
        card.child(
            div()
                .flex()
                .items_center()
                .child(div().flex_1().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child("Targets"))
                .child(div().id("snippet-edit-targets").text_sm().font_weight(FontWeight::BOLD).text_color(rgb(theme::ACCENT)).child("Edit").on_click(cx.listener(move |shell, _, window, cx| open_picker(shell, window, cx)))),
        )
        .children(targets.into_iter().map(|h| host_row(format!("target-{}", h.id), host_tile(h), h.display_label().to_owned(), h.subtitle()).bg(rgb(theme::BACKGROUND))))
    }

    // ── Targets picker ──────────────────────────────────────────────────

    fn render_target_picker(&self, editor: &SnippetEditor, focused: bool, cx: &mut Context<Self>) -> Div {
        let adding = editor.route == Route::AddTargets;
        let back = div()
            .id("targets-back")
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(rgb(theme::SURFACE_ACTIVE))
            .child(svg().path(icon("arrow_left")).size(px(12.)).text_color(rgb(theme::TEXT)))
            .on_click(cx.listener(move |shell, _, _, cx| {
                if let Some(editor) = &mut shell.snippet_editor {
                    if adding {
                        editor.route = Route::Edit;
                        editor.field = Field::Label;
                    } else {
                        shell.snippet_editor = None;
                    }
                }
                cx.notify();
            }));
        let head = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .gap_3()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(back)
            .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(if adding { "Add targets" } else { "Select targets" }));

        let searching = !editor.search.trim().is_empty();
        let search = field_box(focused && editor.field == Field::Search)
            .id("targets-search")
            .child(svg().path(icon("search")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(input_text(&editor.search, "Search", focused && editor.field == Field::Search))
            .on_click(cx.listener(|shell, _, window, cx| shell.focus_snippet_field(Field::Search, window, cx)));

        let mut list = div().flex().flex_col();
        if !searching {
            if let Some(group) = editor.picker_group {
                let mut crumbs = div().pb_2().flex().items_center().gap_2().text_sm().child(
                    div().id("targets-crumb-root").text_color(rgb(theme::ACCENT)).child("All hosts").on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.snippet_editor {
                            editor.picker_group = None;
                        }
                        cx.notify();
                    })),
                );
                for g in self.vault.breadcrumbs(Some(group)) {
                    let target = g.id;
                    crumbs = crumbs.child(div().text_color(rgb(theme::TEXT_MUTED)).child("›")).child(
                        div().id(SharedString::from(format!("targets-crumb-{target}"))).text_color(rgb(if target == group { theme::TEXT } else { theme::ACCENT })).child(g.label.clone()).on_click(cx.listener(move |shell, _, _, cx| {
                            if let Some(editor) = &mut shell.snippet_editor {
                                editor.picker_group = Some(target);
                            }
                            cx.notify();
                        })),
                    );
                }
                list = list.child(crumbs);
            }
            let groups = self.vault.child_groups(editor.picker_group);
            if !groups.is_empty() {
                list = list.child(div().px(px(10.)).pb(px(5.)).font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child("Groups"));
                for group in groups {
                    let id = group.id;
                    let count = self.vault.hosts_under(Some(id)).len();
                    let state = snippets::coverage(&self.vault, id, &editor.picked);
                    let check = div().id(SharedString::from(format!("targets-group-check-{id}"))).p_1().child(checkbox(state)).on_click(cx.listener(move |shell, _, _, cx| {
                        cx.stop_propagation();
                        if let Some(editor) = &mut shell.snippet_editor {
                            snippets::toggle_group(&shell.vault, id, &mut editor.picked);
                        }
                        cx.notify();
                    }));
                    let row = host_row(format!("targets-group-{id}"), tile("group", theme::TILE_BLUE), group.label.clone(), format!("{count} {}", if count == 1 { "Host" } else { "Hosts" }))
                        .hover(|s| s.bg(rgb(theme::SURFACE)))
                        .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)))
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            if let Some(editor) = &mut shell.snippet_editor {
                                editor.picker_group = Some(id);
                            }
                            cx.notify();
                        }));
                    list = list.child(div().flex().items_center().gap_1().child(check).child(row.flex_1()));
                }
            }
        }
        let hosts = if searching { self.vault.search(&editor.search) } else { self.vault.hosts_under(editor.picker_group) };
        if !hosts.is_empty() {
            list = list.child(div().px(px(10.)).pt(px(15.)).pb(px(5.)).font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child("Hosts"));
            for host in hosts {
                let id = host.id;
                let state = if editor.picked.contains(&id) { Coverage::All } else { Coverage::None };
                let row = host_row(format!("targets-host-{id}"), host_tile(host), host.display_label().to_owned(), host.subtitle()).flex_1();
                list = list.child(
                    div()
                        .id(SharedString::from(format!("targets-pick-{id}")))
                        .flex()
                        .items_center()
                        .gap_1()
                        .rounded_lg()
                        .hover(|s| s.bg(rgb(theme::SURFACE)))
                        .child(div().p_1().child(checkbox(state)))
                        .child(row)
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            if let Some(editor) = &mut shell.snippet_editor {
                                snippets::toggle_host(id, &mut editor.picked);
                            }
                            cx.notify();
                        })),
                );
            }
        } else if self.vault.hosts.is_empty() {
            list = list.child(empty_state("hosts", "No hosts yet", "Add hosts on the Hosts screen to run snippets on them."));
        }

        let footer: Option<AnyElement> = if adding {
            (!self.vault.hosts.is_empty()).then(|| {
                dialog::button("targets-save", "Save", Kind::Accent, false)
                    .w_full()
                    .on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.snippet_editor {
                            editor.snippet.targets = std::mem::take(&mut editor.picked);
                            editor.route = Route::Edit;
                            editor.field = Field::Label;
                        }
                        shell.save_snippet_draft();
                        cx.notify();
                    }))
                    .into_any_element()
            })
        } else {
            let count = editor.picked.len();
            let enabled = (1..=MAX_TARGETS).contains(&count);
            let toggle = dialog::checkbox("targets-save-toggle", "Save targets", self.snippet_save_targets).text_sm().text_color(rgb(theme::TEXT)).on_click(cx.listener(|shell, _, _, cx| {
                shell.snippet_save_targets = !shell.snippet_save_targets;
                cx.notify();
            }));
            let run = dialog::button("targets-run", "Run", Kind::Accent, !enabled).w_full().when(enabled, |d| {
                d.on_click(cx.listener(|shell, _, window, cx| {
                    let Some(editor) = &mut shell.snippet_editor else { return };
                    let targets = editor.picked.clone();
                    let script = editor.snippet.script.clone();
                    let title = editor.snippet.display_label().to_owned();
                    if shell.snippet_save_targets {
                        editor.snippet.targets = targets.clone();
                        shell.save_snippet_draft();
                    }
                    shell.run_snippet(&title, &script, &targets, window, cx);
                }))
            });
            Some(div().flex().flex_col().gap(px(15.)).child(toggle).child(run).into_any_element())
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(head)
            .child(div().flex_none().p(px(15.)).child(search))
            .child(div().id("targets-scroll").flex_1().overflow_y_scroll().px(px(10.)).child(list))
            .children(footer.map(|f| div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).child(f)))
    }

    // ── Package slider ──────────────────────────────────────────────────

    pub fn new_package(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let package = SnippetPackage { id: self.vault.next_id(), label: String::new() };
        self.snippet_editor = None;
        self.snippet_menu = None;
        self.shell_history = None;
        self.package_editor = Some(PackageEditor { package, created_here: true, menu_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    pub fn edit_package(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(package) = self.vault.package(id).cloned() else { return };
        self.snippet_editor = None;
        self.snippet_menu = None;
        self.shell_history = None;
        self.package_editor = Some(PackageEditor { package, created_here: false, menu_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn on_package_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(editor) = &mut self.package_editor else { return };
        if editor.confirm_remove {
            if event.keystroke.key == "escape" {
                editor.confirm_remove = false;
                cx.notify();
            }
            return;
        }
        match dialog::edit(Some(&mut editor.package.label), &event.keystroke, paste) {
            // Saved as typed once it has a label (the legacy form requires one).
            Key::Edited if !editor.package.label.trim().is_empty() => {
                self.vault.save_package(editor.package.clone());
                self.save_vault();
            }
            Key::Edited | Key::Enter => {}
            Key::Escape => self.package_editor = None,
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub fn render_package_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.package_editor.as_ref()?;
        let id = editor.package.id;
        let saved = self.vault.package(id).is_some();
        let focused = self.editor_focus.is_focused(window);
        let head = header(if editor.created_here && !saved { "New Package" } else { "Edit Package" })
            .when(saved, |d| d.child(svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::TEXT))))
            .child(dots("package-dots").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.package_editor {
                    editor.menu_open = !editor.menu_open;
                }
                cx.notify();
            })))
            .child(dialog::close_button("package-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.package_editor = None;
                cx.notify();
            })));
        let label = field_box(focused).id("package-label").flex_1().child(input_text(&editor.package.label, "Label *", focused)).on_click(cx.listener(|shell, _, window, cx| {
            window.focus(&shell.editor_focus);
            cx.notify();
        }));
        let form = div()
            .p(px(15.))
            .pb(px(20.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .child(div().flex().items_center().gap_3().child(tile("snippets_package", theme::TILE_BLUE).size(px(36.)).rounded_lg()).child(label));
        let menu = editor.menu_open.then(|| {
            div().absolute().top(px(56.)).right(px(50.)).child(menu_panel("package-menu").child(menu_row("package-menu-remove".into(), "Remove", true, !saved).when(saved, |d| {
                d.on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.package_editor {
                        editor.menu_open = false;
                        editor.confirm_remove = true;
                    }
                    cx.notify();
                }))
            })))
        });
        let confirm = editor.confirm_remove.then(|| {
            let close = dialog::close_button("package-remove-close").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.package_editor {
                    editor.confirm_remove = false;
                }
                cx.notify();
            }));
            self.remove_card(&[id], &[], close, cx)
        });
        let slider = slider("package-editor")
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_package_key))
            .child(head)
            .child(div().flex_1().p(px(15.)).child(form))
            .children(menu);
        Some(match confirm {
            Some(confirm) => div().absolute().inset_0().child(slider).child(confirm).into_any_element(),
            None => slider.into_any_element(),
        })
    }

    // ── Card menu and removal ───────────────────────────────────────────

    fn remove_items(&mut self, packages: &[u64], snippets: &[u64], cx: &mut Context<Self>) {
        for id in snippets {
            self.vault.remove_snippet(*id);
        }
        for id in packages {
            self.vault.remove_package(*id);
        }
        if self.snippet_package.is_some_and(|p| self.vault.package(p).is_none()) {
            self.snippet_package = None;
        }
        self.save_vault();
        self.close_snippet_sliders();
        cx.notify();
    }

    fn remove_card(&self, packages: &[u64], snippets: &[u64], close: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        let (title, text) = snippets::remove_texts(&self.vault, packages, snippets);
        let (packages, snippets) = (packages.to_vec(), snippets.to_vec());
        dialog::backdrop(dialog::card(&title, false, close, dialog::text(&text), vec![dialog::button("snippets-remove-confirm", "Remove", Kind::Error, false)
            .on_click(cx.listener(move |shell, _, _, cx| shell.remove_items(&packages, &snippets, cx)))
            .into_any_element()]))
    }

    fn dismiss_snippet_menu(&mut self, _: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.snippet_menu = None;
        cx.notify();
    }

    fn ask_remove_from_menu(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = &mut self.snippet_menu {
            menu.confirm_remove = true;
        }
        cx.notify();
    }

    pub fn render_snippet_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.snippet_menu.as_ref()?;
        if menu.confirm_remove {
            let close = dialog::close_button("snippets-remove-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.snippet_menu = None;
                cx.notify();
            }));
            let (packages, snippets) = match menu.item {
                MenuItem::Package(id) => (vec![id], vec![]),
                MenuItem::Snippet(id) => (vec![], vec![id]),
                MenuItem::New => return None,
            };
            return Some(self.remove_card(&packages, &snippets, close, cx).into_any_element());
        }
        let items = match menu.item {
            MenuItem::Snippet(id) => menu_panel("snippet-card-menu")
                .child(
                    menu_row("snippet-menu-run".into(), "Run", false, false)
                        .child(div().flex_1())
                        .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Double Click"))
                        .on_click(cx.listener(move |shell, _, window, cx| shell.execute_snippet(id, window, cx))),
                )
                .child(menu_row("snippet-menu-edit".into(), "Edit", false, false).on_click(cx.listener(move |shell, _, window, cx| shell.edit_snippet(id, window, cx))))
                .child(menu_row("snippet-menu-duplicate".into(), "Duplicate", false, false).on_click(cx.listener(move |shell, _, _, cx| {
                    shell.snippet_menu = None;
                    if shell.vault.duplicate_snippet(id).is_some() {
                        shell.save_vault();
                    }
                    cx.notify();
                })))
                .child(menu_row("snippet-menu-remove".into(), "Remove", true, false).on_click(cx.listener(Self::ask_remove_from_menu))),
            MenuItem::Package(id) => menu_panel("package-card-menu")
                .child(menu_row("package-menu-edit".into(), "Edit", false, false).on_click(cx.listener(move |shell, _, window, cx| shell.edit_package(id, window, cx))))
                .child(menu_row("package-card-menu-remove".into(), "Remove", true, false).on_click(cx.listener(Self::ask_remove_from_menu))),
            MenuItem::New => menu_panel("new-snippet-menu-panel").child(menu_row("new-snippet-package".into(), "New snippet package", false, false).on_click(cx.listener(|shell, _, window, cx| shell.new_package(window, cx)))),
        };
        let viewport = window.viewport_size();
        let left = menu.position.x.min(viewport.width - px(230.));
        let top = menu.position.y.min(viewport.height - px(150.));
        Some(
            gpui::deferred(
                div()
                    .id("snippet-menu-dismiss")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::dismiss_snippet_menu))
                    .on_mouse_down(MouseButton::Right, cx.listener(Self::dismiss_snippet_menu))
                    .child(gpui::anchored().position(Point { x: left, y: top }).child(items)),
            )
            .into_any_element(),
        )
    }

    /// Whatever covers the Snippets screen: a slider, a menu or a dialog.
    pub fn render_snippets_overlay(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let slider = self.render_snippet_editor(window, cx).or_else(|| self.render_package_editor(window, cx)).or_else(|| self.render_shell_history(cx));
        match (slider, self.render_snippet_menu(window, cx)) {
            (Some(slider), Some(menu)) => Some(div().absolute().inset_0().child(slider).child(menu).into_any_element()),
            (slider, menu) => slider.or(menu),
        }
    }

    // ── Shell History ───────────────────────────────────────────────────

    fn on_shell_history_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(panel) = &mut self.shell_history else { return };
        let Some((row, label)) = &mut panel.editing else {
            if event.keystroke.key == "escape" {
                self.shell_history = None;
                cx.notify();
            }
            return;
        };
        match dialog::edit(Some(label), &event.keystroke, paste) {
            Key::Enter => {
                let row = *row;
                self.save_history_command(row);
            }
            Key::Escape => panel.editing = None,
            Key::Edited => {}
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// «Done»: the command becomes a snippet with the typed label.
    fn save_history_command(&mut self, row: usize) {
        let Some(panel) = &mut self.shell_history else { return };
        let label = panel.editing.take().map(|(_, l)| l).unwrap_or_default();
        let Some(command) = self.vault.shell_history.get(row) else { return };
        let snippet = Snippet { id: self.vault.next_id(), label, script: command.script.clone(), ..Snippet::default() };
        self.vault.save_snippet(snippet);
        panel.saved.push(row);
        self.save_vault();
    }

    fn render_shell_history(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = self.shell_history.as_ref()?;
        let head = header("Shell History")
            .child(dots("history-dots").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(panel) = &mut shell.shell_history {
                    panel.menu_open = !panel.menu_open;
                }
                cx.notify();
            })))
            .child(dialog::close_button("history-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.shell_history = None;
                cx.notify();
            })));
        let commands = &self.vault.shell_history;
        let body = if commands.is_empty() {
            div().child(empty_state("logs", "You don't have shell history yet", "Connect to a host via SSH to see your shell history here."))
        } else {
            div().flex().flex_col().children(commands.iter().enumerate().map(|(i, command)| {
                let hover = SharedString::from(format!("history-row-{i}"));
                let expanded = panel.expanded == Some(i);
                let editing = panel.editing.as_ref().filter(|(row, _)| *row == i).map(|(_, label)| label.clone());
                let saved = panel.saved.contains(&i);
                let action = |id: String, text: &str| {
                    div().id(SharedString::from(id)).h(px(22.)).w(px(52.)).flex().items_center().justify_center().rounded_md().bg(rgb(theme::ACCENT)).text_xs().font_weight(FontWeight::BLACK).text_color(rgb(0xffffff)).child(text.to_owned())
                };
                let end = if editing.is_some() {
                    Some(action(format!("history-done-{i}"), "Done").on_click(cx.listener(move |shell, _, _, cx| {
                        cx.stop_propagation();
                        shell.save_history_command(i);
                        cx.notify();
                    })).into_any_element())
                } else if saved {
                    Some(div().flex().items_center().gap_1().text_xs().text_color(rgb(theme::ACCENT)).child(svg().path(icon("check")).size(px(8.)).text_color(rgb(theme::ACCENT))).child("saved").into_any_element())
                } else {
                    Some(div().invisible().group_hover(hover.clone(), |s| s.visible()).child(action(format!("history-save-{i}"), "SAVE").on_click(cx.listener(move |shell, _, window, cx| {
                        cx.stop_propagation();
                        if let Some(panel) = &mut shell.shell_history {
                            panel.editing = Some((i, String::new()));
                        }
                        window.focus(&shell.editor_focus);
                        cx.notify();
                    }))).into_any_element())
                };
                let title = match &editing {
                    Some(label) => input_text(label, "Set a label", true),
                    None => div().text_color(rgb(theme::TEXT)).font_family("Menlo").text_sm().when(!expanded, |d| d.truncate()).child(command.script.clone()),
                };
                div()
                    .id(hover.clone())
                    .group(hover)
                    .px(px(15.))
                    .py(px(10.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .hover(|s| s.bg(rgb(theme::SURFACE)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(title)
                            .when(editing.is_some() || expanded, |d| d.child(div().font_family("Menlo").text_xs().text_color(rgb(theme::TEXT_MUTED)).child(command.script.clone()))),
                    )
                    .children(end)
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(panel) = &mut shell.shell_history {
                            if panel.editing.is_none() {
                                panel.expanded = if panel.expanded == Some(i) { None } else { Some(i) };
                            }
                        }
                        cx.notify();
                    }))
            }))
        };
        let menu = panel.menu_open.then(|| {
            div().absolute().top(px(56.)).right(px(50.)).child(menu_panel("history-menu").child(menu_row("history-delete-all".into(), "Delete All", true, false).on_click(cx.listener(|shell, _, _, cx| {
                shell.vault.shell_history.clear();
                shell.save_vault();
                shell.shell_history = Some(ShellHistoryPanel::default());
                cx.notify();
            }))))
        });
        Some(
            slider("shell-history")
                .track_focus(&self.editor_focus)
                .on_key_down(cx.listener(Self::on_shell_history_key))
                .child(head)
                .child(div().id("history-scroll").flex_1().overflow_y_scroll().child(body))
                .children(menu)
                .into_any_element(),
        )
    }
}
