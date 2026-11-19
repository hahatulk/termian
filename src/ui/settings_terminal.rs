//! Settings → Terminal (legacy `index-7ecae23c`): the switches, emulation
//! type, font and text size, theme, keyword highlighting, OS detection,
//! local terminal path, keepalive, scrollback and post-quantum key exchange.
//! Changes reach open terminals at once; new connections pick up the rest.

use std::borrow::Cow;

use gpui::{div, prelude::*, px, rgb, rgba, svg, AnyElement, App, Bounds, Context, Div, Element, ElementId, FontWeight, GlobalElementId, InspectorElementId, LayoutId, Pixels, SharedString, Stateful, Window};
use termian_native::terminal_settings::{self as ts, TerminalSettings, EMULATION_TYPES, FONT_FAMILIES, HIGHLIGHT_GROUPS, MAX_FONT_SIZE, MIN_FONT_SIZE};
use termian_native::theme;

use super::settings::{body_text, section, Field, Menu, SettingsWindow, MUTED};
use super::text_input::{self, Mode};
use crate::Shell;

/// The legacy «Detect OS Script», shown read-only.
const DETECT_OS_SCRIPT: &str = "# get the shell name to choose the script for detecting OS
  echo $SHELL

# for routeros
  :put [/system resource get platform]

# for fish shell
  if set name (uname) = \"Linux\"
      cat /etc/*release
  else
      uname
  end

# for others
  HISTFILE=;
  SA_OS_TYPE=\"Linux\"
  REAL_OS_NAME=`uname`
  if [ \"$REAL_OS_NAME\" != \"$SA_OS_TYPE\" ] ;
  then
  echo `uname`
  else
  DISTRIB_ID=\\\"`cat /etc/*release`\\\"
  echo $DISTRIB_ID;
  fi;
  exit;";

impl Shell {
    /// Changes Settings → Terminal and applies it to every open terminal.
    pub fn update_terminal(&mut self, change: impl FnOnce(&mut TerminalSettings), cx: &mut Context<Self>) {
        change(&mut self.vault.terminal);
        self.save_vault();
        self.load_terminal_font(cx);
        let settings = self.vault.terminal.clone();
        for view in self.all_views() {
            view.update(cx, |view, cx| view.apply_settings(&settings, cx));
        }
        cx.notify();
    }

    /// «Keepalive Interval» goes into the synced snapshot settings, as legacy keeps it.
    pub fn set_keep_alive(&mut self, seconds: u32, cx: &mut Context<Self>) {
        if ts::keep_alive(&self.vault.foreign) != seconds {
            ts::set_keep_alive(&mut self.vault.foreign, seconds);
            self.save_vault();
            cx.notify();
        }
    }

    /// Registers the bundled files of the chosen font with the text system, once.
    pub fn load_terminal_font(&mut self, cx: &mut App) {
        let face = ts::font_face(&self.vault.terminal.font_family);
        if face.files.is_empty() || !self.loaded_fonts.insert(face.label) {
            return;
        }
        let files: Vec<Cow<'static, [u8]>> = ts::font_files(&face).into_iter().map(Cow::Owned).collect();
        if files.is_empty() {
            eprintln!("[Termian] Font files of {} were not found", face.label);
        } else if let Err(error) = cx.text_system().add_fonts(files) {
            eprintln!("[Termian] Font {} could not be loaded: {error}", face.label);
        }
    }
}

fn heading(text: &str) -> Div {
    div().text_size(px(14.)).font_weight(FontWeight::BOLD).line_height(px(24.)).text_color(rgb(theme::TEXT)).child(text.to_owned())
}

fn label(text: &str) -> Div {
    div().text_size(px(12.)).font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child(text.to_owned())
}

pub(super) fn toggle(on: bool) -> impl IntoElement {
    Switch(on)
}

struct Switch(bool);

impl IntoElement for Switch {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Switch {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some("switch-motion".into())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let target = if self.0 { 1. } else { 0. };
        let t = window.with_element_state(id.expect("switch element id"), |state, window| {
            let mut tween = state.unwrap_or_else(|| super::motion::Tween::at(target));
            (tween.toward(target, 190, window), tween)
        });
        let mut accent = rgb(theme::ACCENT);
        accent.a = t;
        let mut element = div()
            .w(px(36.))
            .h(px(20.))
            .flex_none()
            .p(px(2.))
            .rounded_full()
            .flex()
            .bg(rgb(0x4a4e66).blend(accent))
            .child(div().size(px(16.)).ml(px(t * 16.)).rounded_full().bg(rgb(0xffffff)))
            .into_any_element();
        (element.request_layout(window, cx), element)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

/// `SettingsSwitch`: the label on the left, the switch on the right.
fn switch(id: &str, text: &str, on: bool) -> Stateful<Div> {
    div().id(SharedString::from(id.to_owned())).mt(px(15.)).flex().items_center().justify_between().gap(px(10.)).cursor_pointer().child(label(text)).child(toggle(on))
}

/// A value with a drop-down arrow, like the legacy `FiltersButton`.
pub(super) fn picker(id: &str, value: &str, open: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .flex()
        .items_center()
        .gap(px(5.))
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(theme::TEXT))
        .cursor_pointer()
        .child(value.to_owned())
        .child(arrow(open))
}

/// The legacy drop-down arrow: a caret that turns up while the menu is open.
pub(super) fn arrow(open: bool) -> impl IntoElement {
    let caret = svg().path(super::assets::icon("caret_down")).w(px(11.)).h(px(7.)).text_color(rgb(theme::TEXT));
    if open { caret.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5))) } else { caret }
}

pub(super) fn menu_list(children: Vec<AnyElement>) -> Div {
    div().mt(px(6.)).p(px(5.)).rounded(px(15.)).border_1().border_color(rgba(0xffffff1a)).bg(rgb(0x2f3348)).max_h(px(260.)).overflow_hidden().children(children)
}

pub(super) fn menu_item(id: SharedString, text: &str, selected: bool) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .px(px(10.))
        .py(px(6.))
        .rounded(px(10.))
        .flex()
        .justify_between()
        .text_size(px(14.))
        .text_color(rgb(theme::TEXT))
        .cursor_pointer()
        .hover(|s| s.bg(rgba(0xffffff14)))
        .child(div().flex_1().child(text.to_owned()))
        .when(selected, |d| d.child(div().text_color(rgb(MUTED)).child("✓")))
}

impl SettingsWindow {
    fn terminal(&self, cx: &App) -> TerminalSettings {
        self.shell.read(cx).vault.terminal.clone()
    }

    fn change(&self, cx: &mut Context<Self>, change: impl FnOnce(&mut TerminalSettings) + 'static) {
        self.shell.update(cx, |shell, cx| shell.update_terminal(change, cx));
    }

    /// A text field; `shown` is what it holds when not being typed into.
    fn text_field(&self, field: Field, id: &str, shown: String, suffix: Option<String>, width: Option<f32>, cx: &mut Context<Self>) -> Stateful<Div> {
        let typing = self.editing.as_ref().filter(|(f, _)| *f == field).map(|(_, text)| text.clone());
        let focused = typing.is_some();
        let text = typing.unwrap_or(shown.clone());
        let key = ("settings", &field);
        let input = div()
            .id(SharedString::from(id.to_owned()))
            .h(px(36.)).px(px(10.)).flex().items_center().rounded(px(6.)).border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { 0x4a4e66 }))
            .bg(rgb(theme::INPUT)).text_size(px(14.)).text_color(rgb(theme::TEXT))
            .when_some(width, |d, w| d.w(px(w)).justify_center())
            .when(width.is_none(), |d| d.w_full())
            .child(text_input::text(key, &text, "", focused, Mode::Line, cx))
            .when_some(suffix, |d, suffix| d.child(div().ml(px(3.)).text_color(rgb(MUTED)).child(suffix)));
        text_input::field(input, key, Mode::Line, {
            let field = field.clone();
            move |settings: &mut Self, window, cx| {
                if settings.editing.as_ref().is_none_or(|(active, _)| *active != field) {
                    settings.commit_edit(cx);
                    settings.editing = Some((field.clone(), shown.clone()));
                }
                window.focus(&settings.focus);
            }
        }, cx)

    }

    /// Typing changes Text Size and Keepalive at once, as the legacy fields do.
    pub(super) fn live_edit(&mut self, cx: &mut Context<Self>) {
        let Some((field, text)) = self.editing.clone() else { return };
        match field {
            Field::FontSize => {
                let digits: String = text.chars().filter(char::is_ascii_digit).collect();
                if let Some((_, typed)) = &mut self.editing {
                    *typed = digits.clone();
                }
                if let Ok(size) = digits.parse::<u32>() {
                    if size >= MIN_FONT_SIZE {
                        let size = size.min(MAX_FONT_SIZE);
                        if let Some((_, typed)) = &mut self.editing {
                            *typed = size.to_string();
                        }
                        self.change(cx, move |t| t.set_font_size(size));
                    }
                }
            }
            Field::KeepAlive => {
                let seconds = ts::parse_keep_alive(&text);
                self.shell.update(cx, |shell, cx| shell.set_keep_alive(seconds, cx));
            }
            _ => {}
        }
    }

    pub(super) fn commit_terminal_field(&mut self, field: Field, text: &str, cx: &mut Context<Self>) {
        match field {
            Field::Scrollback => {
                let digits: String = text.trim().trim_start_matches('-').chars().filter(char::is_ascii_digit).collect();
                let value = digits.parse::<u64>().unwrap_or(0);
                self.change(cx, move |t| t.set_scrollback_field(value));
            }
            Field::Shell => {
                let path = text.trim().to_owned();
                self.menu = None;
                self.change(cx, move |t| t.add_shell(&path));
            }
            Field::Font => {
                let family = text.trim().to_owned();
                self.menu = None;
                if !family.is_empty() {
                    self.change(cx, move |t| t.font_family = family);
                }
            }
            Field::FontSize | Field::KeepAlive | Field::App(_) => {}
            Field::SchemeName => self.rename_scheme(text, cx),
        }
    }

    fn toggle_menu(&mut self, menu: Menu, cx: &mut Context<Self>) {
        self.commit_edit(cx);
        self.menu = if self.menu == Some(menu) { None } else { Some(menu) };
        cx.notify();
    }

    fn switch_row(&self, id: &str, text: &str, on: bool, set: fn(&mut TerminalSettings, bool), cx: &mut Context<Self>) -> AnyElement {
        switch(id, text, on).on_click(cx.listener(move |settings, _, _, cx| settings.change(cx, move |t| set(t, !on)))).into_any_element()
    }

    pub(super) fn render_terminal(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = self.terminal(cx);
        let keep_alive = ts::keep_alive(&self.shell.read(cx).vault.foreign);

        // Terminal settings
        let mut general = vec![
            heading("Terminal settings").into_any_element(),
            div()
                .mt(px(15.))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(label("Autocomplete"))
                        .child(div().px(px(5.)).rounded(px(4.)).border_1().border_color(rgb(MUTED)).text_size(px(10.)).font_weight(FontWeight::BOLD).text_color(rgb(MUTED)).child("BETA")),
                )
                .child(
                    picker("term-autocomplete", if t.autocomplete { "Enabled" } else { "Disabled" }, self.menu == Some(Menu::Autocomplete))
                        .on_click(cx.listener(|settings, _, _, cx| settings.toggle_menu(Menu::Autocomplete, cx))),
                )
                .into_any_element(),
        ];
        if self.menu == Some(Menu::Autocomplete) {
            general.push(
                menu_list(
                    [("Enabled", true), ("Disabled", false)]
                        .into_iter()
                        .map(|(text, on)| {
                            menu_item(SharedString::from(format!("term-autocomplete-{text}")), text, on == t.autocomplete)
                                .on_click(cx.listener(move |settings, _, _, cx| {
                                    settings.menu = None;
                                    settings.change(cx, move |t| t.autocomplete = on);
                                }))
                                .into_any_element()
                        })
                        .collect(),
                )
                .into_any_element(),
            );
        }
        general.extend([
            self.switch_row("term-autoreconnect", "Autoreconnect", t.autoreconnect, |t, v| t.autoreconnect = v, cx),
            self.switch_row("term-history", "Import shell history", t.shell_history_import, |t, v| t.shell_history_import = v, cx),
            self.switch_row("term-integration", "Shell integration (record commands)", t.shell_integration, |t, v| t.shell_integration = v, cx),
            self.switch_row("term-copy", "Select text to copy & Right click to paste", t.copy_on_select, |t, v| t.copy_on_select = v, cx),
            self.switch_row("term-bell", "Bell sound", t.bell_sound, |t, v| t.bell_sound = v, cx),
        ]);
        if cfg!(target_os = "macos") {
            general.push(self.switch_row("term-meta", "Use Option as Meta key", t.option_is_meta, |t, v| t.option_is_meta = v, cx));
        }
        general.push(self.switch_row("term-bold", "Use bright colours for bold text", t.brighten_bold, |t, v| t.brighten_bold = v, cx));
        let emulation_open = self.menu == Some(Menu::Emulation);
        general.push(
            div()
                .mt(px(15.))
                .flex()
                .items_center()
                .justify_between()
                .child(label("Terminal emulation type"))
                .child(picker("term-emulation", t.emulation(), emulation_open).on_click(cx.listener(|settings, _, _, cx| settings.toggle_menu(Menu::Emulation, cx))))
                .into_any_element(),
        );
        if emulation_open {
            let current = t.emulation().to_owned();
            general.push(
                menu_list(
                    EMULATION_TYPES
                        .iter()
                        .map(|kind| {
                            let kind = *kind;
                            menu_item(SharedString::from(format!("term-emulation-{kind}")), kind, kind == current)
                                .on_click(cx.listener(move |settings, _, _, cx| {
                                    settings.menu = None;
                                    settings.change(cx, move |t| t.emulation = kind.into());
                                }))
                                .into_any_element()
                        })
                        .collect(),
                )
                .into_any_element(),
            );
        }

        // Font and Text Size
        let font_open = self.menu == Some(Menu::Font);
        // As in legacy: the family in a full-width field, no label.
        let mut font = vec![div()
            .id("term-font")
            .h(px(32.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(4.))
            .border_1()
            .border_color(rgba(0xffffff33))
            .cursor_pointer()
            .text_size(px(13.))
            .text_color(rgb(theme::TEXT))
            .child(t.font_family.clone())
            .child(arrow(font_open))
            .on_click(cx.listener(|settings, _, _, cx| settings.toggle_menu(Menu::Font, cx)))
            .into_any_element()];
        if font_open {
            let mut items: Vec<AnyElement> = vec![self.text_field(Field::Font, "term-font-custom", String::new(), None, None, cx).into_any_element()];
            items.push(div().mt(px(4.)).px(px(10.)).text_size(px(11.)).text_color(rgb(MUTED)).child("Введите имя шрифта и нажмите Enter, или выберите из списка").into_any_element());
            items.extend(FONT_FAMILIES.iter().map(|family| {
                let family = *family;
                menu_item(SharedString::from(format!("term-font-{family}")), family, family == t.font_family)
                    .on_click(cx.listener(move |settings, _, _, cx| {
                        settings.menu = None;
                        settings.editing = None;
                        settings.change(cx, move |t| t.font_family = family.into());
                    }))
                    .into_any_element()
            }));
            font.push(div().id("term-font-list").max_h(px(320.)).overflow_y_scroll().child(menu_list(items).max_h(px(1000.))).into_any_element());
        }
        let size = t.font_size;
        let step = |id: &str, text: &str, disabled: bool| {
            div()
                .id(SharedString::from(id.to_owned()))
                .size(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(rgb(0x3e4257))
                .text_size(px(16.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::TEXT))
                .when(disabled, |d| d.opacity(0.4))
                .when(!disabled, |d| d.cursor_pointer())
                .child(text.to_owned())
        };
        font.push(
            div()
                .mt(px(15.))
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(px(14.)).text_color(rgb(theme::TEXT)).child("Text Size"))
                .child(
                    div()
                        .flex()
                        .gap(px(5.))
                        .child(step("term-size-down", "−", size <= MIN_FONT_SIZE).when(size > MIN_FONT_SIZE, |d| {
                            d.on_click(cx.listener(move |settings, _, _, cx| {
                                settings.editing = None;
                                settings.change(cx, move |t| t.set_font_size(size - 1));
                            }))
                        }))
                        .child(self.text_field(Field::FontSize, "term-size", size.to_string(), None, Some(51.), cx))
                        .child(step("term-size-up", "+", size >= MAX_FONT_SIZE).when(size < MAX_FONT_SIZE, |d| {
                            d.on_click(cx.listener(move |settings, _, _, cx| {
                                settings.editing = None;
                                settings.change(cx, move |t| t.set_font_size(size + 1));
                            }))
                        })),
                )
                .into_any_element(),
        );

        // Terminal theme
        let current = t.theme().name.clone();
        let themes = ts::themes().iter().map(|theme| {
            let name = theme.name.clone();
            let selected = name == current;
            let key = name.to_lowercase();
            div()
                .id(SharedString::from(format!("term-theme-{key}")))
                .p(px(4.))
                .rounded(px(10.))
                .border_2()
                .border_color(if selected { rgb(theme::ACCENT) } else { rgba(0x00000000) })
                .cursor_pointer()
                .child(
                    div()
                        .h(px(54.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .bg(rgb(theme.background))
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(px(6.))
                        .child(div().text_size(px(12.)).text_color(rgb(theme.foreground)).truncate().child(name.clone()))
                        .child(div().flex().gap(px(3.)).children([2usize, 1, 3, 4, 5, 6].map(|i| div().size(px(8.)).rounded_full().bg(rgb(theme.palette[i]))))),
                )
                .on_click(cx.listener(move |settings, _, _, cx| {
                    let key = key.clone();
                    settings.change(cx, move |t| t.color_scheme = key);
                }))
        });
        let theme_section = vec![
            heading("Terminal theme").into_any_element(),
            div().mt(px(10.)).grid().grid_cols(3).gap(px(6.)).children(themes).into_any_element(),
        ];

        // Keyword highlighting
        let colors = ts::highlight_colors(&current);
        let mut keywords = vec![div()
            .id("term-keywords")
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .child(heading("Keyword highlighting"))
            .child(toggle(t.keyword_highlighting))
            .on_click({
                let on = t.keyword_highlighting;
                cx.listener(move |settings, _, _, cx| settings.change(cx, move |t| t.keyword_highlighting = !on))
            })
            .into_any_element()];
        keywords.push(
            div()
                .mt(px(5.))
                .children(HIGHLIGHT_GROUPS.iter().zip(colors).map(|(group, color)| {
                    div().mt(px(15.)).flex().items_center().justify_between().child(label(group)).child(div().w(px(50.)).h(px(25.)).rounded(px(6.)).bg(rgb(color)))
                }))
                .into_any_element(),
        );
        // Legacy routes this to the Termius feedback board, which the bundle
        // lacks; here it opens the project's issues.
        keywords.push(body_text("Would you like to add custom rules to highlight output in the terminal?").into_any_element());
        keywords.push(
            div()
                .mt(px(10.))
                .child(
                    super::settings::action_button("term-keywords-feedback", "Let us know", true, false)
                        .on_click(|_, _, cx| cx.open_url("https://github.com/hahatulk/termian/issues")),
                )
                .into_any_element(),
        );

        // Detect Host Operating System
        let detect = vec![
            heading("Detect Host Operating System").into_any_element(),
            body_text("This option will execute a script when connecting to a host for the first time to detect its operating system. This will allow Termius to show a corresponding OS icon in the list of hosts.").into_any_element(),
            body_text("The script will be executed only if server didn't return anything about its OS.").into_any_element(),
            self.switch_row("term-detect", "Detect OS", t.detect_os, |t, v| t.detect_os = v, cx),
            div()
                .mt(px(15.))
                .px(px(10.))
                .rounded(px(6.))
                .bg(rgb(0x353950))
                .child(
                    div()
                        .id("term-script")
                        .py(px(10.))
                        .flex()
                        .justify_between()
                        .cursor_pointer()
                        .child(heading("Detect OS Script"))
                        .child(div().text_color(rgb(MUTED)).child(if self.script_open { "▼" } else { "▶" }))
                        .on_click(cx.listener(|settings, _, _, cx| {
                            settings.script_open = !settings.script_open;
                            cx.notify();
                        })),
                )
                .when(self.script_open, |d| {
                    d.child(div().pb(px(10.)).font_family("Menlo").text_size(px(12.)).text_color(rgb(MUTED)).children(DETECT_OS_SCRIPT.lines().map(|line| div().child(line.to_owned()))))
                })
                .into_any_element(),
        ];

        // Local Terminal Path
        let shell_open = self.menu == Some(Menu::Shell);
        let selected_shell = t.local_shell.clone().unwrap_or_else(ts::default_shell);
        let mut shells = vec![
            heading("Local Terminal Path").into_any_element(),
            div()
                .id("term-shell")
                .mt(px(20.))
                .pb(px(4.))
                .flex()
                .justify_between()
                .border_b_1()
                .border_color(rgb(0x4a4e66))
                .cursor_pointer()
                .child(div().text_size(px(14.)).text_color(rgb(theme::TEXT)).truncate().child(selected_shell.clone()))
                .child(arrow(shell_open))
                .on_click(cx.listener(|settings, _, _, cx| settings.toggle_menu(Menu::Shell, cx)))
                .into_any_element(),
        ];
        if shell_open {
            let mut items: Vec<AnyElement> = ts::detected_shells()
                .into_iter()
                .map(|path| (path, false))
                .chain(t.custom_shells.iter().cloned().map(|path| (path, true)))
                .enumerate()
                .map(|(index, (path, custom))| {
                    let chosen = path.clone();
                    let removed = path.clone();
                    menu_item(SharedString::from(format!("term-shell-{index}")), &path, path == selected_shell)
                        .on_click(cx.listener(move |settings, _, _, cx| {
                            settings.menu = None;
                            let chosen = chosen.clone();
                            settings.change(cx, move |t| t.local_shell = if chosen == ts::default_shell() && !t.custom_shells.contains(&chosen) { None } else { Some(chosen) });
                        }))
                        .when(custom, |d| {
                            d.child(
                                div()
                                    .id(SharedString::from(format!("term-shell-remove-{index}")))
                                    .ml(px(8.))
                                    .text_color(rgb(0xec706f))
                                    .child("✕")
                                    .on_click(cx.listener(move |settings, _, _, cx| {
                                        cx.stop_propagation();
                                        let removed = removed.clone();
                                        settings.change(cx, move |t| t.remove_shell(&removed));
                                    })),
                            )
                        })
                        .into_any_element()
                })
                .collect();
            items.push(div().mt(px(6.)).child(self.text_field(Field::Shell, "term-shell-custom", String::new(), None, None, cx)).into_any_element());
            items.push(div().mt(px(4.)).px(px(10.)).text_size(px(11.)).text_color(rgb(MUTED)).child("Введите путь к оболочке и нажмите Enter, чтобы добавить его").into_any_element());
            shells.push(menu_list(items).max_h(px(1000.)).into_any_element());
        }

        // Keepalive Interval
        let keepalive = vec![
            heading("Keepalive Interval").into_any_element(),
            body_text("How often (in seconds) to send SSH-level keepalive packets to the server. Set to 0 to disable.").into_any_element(),
            div().mt(px(10.)).child(label("Interval")).into_any_element(),
            div()
                .mt(px(6.))
                .child(self.text_field(Field::KeepAlive, "term-keepalive", keep_alive.to_string(), (keep_alive > 0).then(|| if keep_alive > 1 { "seconds".into() } else { "second".into() }), None, cx))
                .into_any_element(),
        ];

        // Scrollback
        let scrollback = vec![
            heading("Scrollback").into_any_element(),
            body_text("Limit number of terminal rows. Set to 0 to maximum limit size.").into_any_element(),
            div().mt(px(10.)).child(label("Number of rows")).into_any_element(),
            div().mt(px(6.)).child(self.text_field(Field::Scrollback, "term-scrollback", t.scrollback_field().to_string(), None, None, cx)).into_any_element(),
        ];

        // Post-Quantum Key Exchange
        let pqc = vec![
            div()
                .id("term-pqc")
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .child(label("Post-Quantum Key Exchange"))
                .child(toggle(t.pqc_kex))
                .on_click({
                    let on = t.pqc_kex;
                    cx.listener(move |settings, _, _, cx| settings.change(cx, move |t| t.pqc_kex = !on))
                })
                .into_any_element(),
            body_text("Turn off if you're experiencing issues with legacy devices").into_any_element(),
        ];

        [section(general), section(font), section(theme_section), section(keywords), section(detect), section(shells), section(keepalive), section(scrollback), section(pqc).mb(px(60.))]
            .into_iter()
            .map(IntoElement::into_any_element)
            .collect()
    }
}
