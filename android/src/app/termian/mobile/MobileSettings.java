package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.app.AlertDialog;
import android.content.Context;
import android.content.SharedPreferences;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import org.json.JSONArray;
import org.json.JSONObject;

final class MobileSettings {
    private final MainActivity app;
    private final MobileUpdates updates;
    private final SharedPreferences preferences;
    private final JSONArray routes, appItems, shortcuts, periods;
    private String active = "";

    MobileSettings(MainActivity app, MobileUpdates updates) {
        this.app = app;
        this.updates = updates;
        preferences = app.getPreferences(Context.MODE_PRIVATE);
        JSONObject menu = app.call("settingsMenu");
        routes = menu.optJSONArray("tabs");
        appItems = menu.optJSONArray("app");
        shortcuts = menu.optJSONArray("shortcuts");
        periods = menu.optJSONArray("logPeriods");
    }

    JSONArray appItems() { return appItems; }

    void open() { active = app.wide() ? "account" : ""; }

    boolean back() {
        if (app.wide() || active.isEmpty()) return false;
        active = "";
        return true;
    }

    String title() {
        if (!app.wide()) for (int i = 0; i < routes.length(); i++) {
            JSONObject route = routes.optJSONObject(i);
            if (active.equals(route.optString("id"))) return text(route.optString("title"));
        }
        return text("Settings");
    }

    View view() {
        updates.detach();
        boolean wide = app.wide();
        String section = wide && active.isEmpty() ? "account" : active;
        if (!wide && section.isEmpty()) return navigation(section);

        LinearLayout page = app.column();
        page.setPadding(app.dp(14), app.dp(14), app.dp(14), app.dp(18));
        page.setFocusableInTouchMode(true);
        switch (section) {
            case "account": account(page); break;
            case "terminal": terminal(page); break;
            case "sftp": app.sftpSettings(card(page, "File type associations")); break;
            case "logs": logs(page); break;
            case "shortcuts": shortcuts(page); break;
            case "version": updates.build(card(page, "Version")); break;
            case "language": language(page); break;
        }
        ScrollView scroll = app.scroll(page);
        scroll.setFillViewport(true);
        if (!wide) return scroll;

        LinearLayout split = new LinearLayout(app);
        split.addView(navigation(section), new LinearLayout.LayoutParams(app.dp(220), -1));
        split.addView(scroll, new LinearLayout.LayoutParams(0, -1, 1));
        return split;
    }

    private View navigation(String selected) {
        LinearLayout entries = app.column();
        entries.setPadding(app.dp(8), app.dp(8), app.dp(8), app.dp(8));
        for (int i = 0; i < routes.length(); i++) {
            JSONObject route = routes.optJSONObject(i);
            String id = route.optString("id");
            entries.addView(app.style.section(icon(id), text(route.optString("title")), id.equals(selected), () -> {
                active = id;
                app.renderSettings();
            }), new LinearLayout.LayoutParams(-1, app.dp(48)));
        }
        ScrollView scroll = app.scroll(entries);
        scroll.setFillViewport(true);
        scroll.setBackgroundColor(TermianStyle.SIDEBAR);
        return scroll;
    }

    private String icon(String id) {
        switch (id) {
            case "account": return "person";
            case "terminal": return "terminal";
            case "sftp": return "sftp";
            case "logs": return "logs";
            case "shortcuts": return "backspace";
            case "version": return "cloud_download";
            case "language": return "env";
            default: return "dots";
        }
    }

    private LinearLayout card(LinearLayout page, String title) {
        LinearLayout card = app.column();
        card.setPadding(app.dp(12), app.dp(8), app.dp(12), app.dp(12));
        card.setBackground(app.style.shape(TermianStyle.SURFACE, 10));
        card.addView(app.label(text(title), 16, TermianStyle.TEXT));
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(-1, -2);
        params.bottomMargin = app.dp(12);
        page.addView(card, params);
        return card;
    }

    private void description(LinearLayout card, String label) {
        card.addView(app.label(text(label), 14, TermianStyle.MUTED));
    }

    private void action(LinearLayout card, String label, Runnable run) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(-1, -2);
        params.topMargin = app.dp(4);
        card.addView(app.button(text(label), run), params);
    }

    private void account(LinearLayout page) {
        JSONObject state = app.call("settingsState");
        LinearLayout local = card(page, "Локальная учётная запись");
        description(local, "Локальный пользователь");
        local.addView(app.label(state.optString("vaultName"), 14, TermianStyle.TEXT));
        action(local, "Privacy password", () -> app.privacyPassword(false));

        JSONObject config = app.call("cloud");
        LinearLayout cloud = card(page, "Своё облако");
        boolean configured = !config.isNull("url");
        if (configured) cloud.addView(app.label(config.optString("url"), 14, TermianStyle.MUTED));
        action(cloud, configured ? "Настройки подключения" : "Подключить сервер", app::editCloudSettings);
        if (configured) {
            action(cloud, "Синхронизировать", () -> app.cloudAction("sync"));
            action(cloud, "Отправить данные на сервер", () -> app.cloudAction("forceUpload"));
            action(cloud, "Получить данные с сервера", () -> app.cloudAction("forceDownload"));
            action(cloud, "Проверить", () -> app.cloudAction("cloudPreview"));
        }

        LinearLayout logs = card(page, "Логи сессий");
        for (int i = 0; i < periods.length(); i++) {
            JSONObject period = periods.optJSONObject(i);
            action(logs, period.optString("title"), () -> new AlertDialog.Builder(app)
                .setTitle(text(period.optString("title"))).setMessage(text("Delete session logs for this period?"))
                .setNegativeButton(text("Cancel"), null).setPositiveButton(text("Delete"), (dialog, which) -> {
                    JSONObject result = app.call("clearLogs", "hours", period.optInt("hours"));
                    app.toast(result.optString(result.has("error") ? "error" : "message"));
                    app.refreshVault();
                }).show());
        }
    }

    private void logs(LinearLayout page) {
        JSONObject state = app.call("settingsState");
        LinearLayout card = card(page, "Logs retention policies");
        description(card, "Control how logs are stored in your vaults.");
        card.addView(app.label(state.optString("vaultName"), 14, TermianStyle.TEXT));
        boolean enabled = state.optBoolean("logsEnabled");
        action(card, enabled ? "Enabled" : "Disabled", () -> choose("Logs", new String[]{"Enabled", "Disabled"}, enabled ? 0 : 1, index -> {
            if (enabled == (index == 0)) return;
            if (index == 0) saveLogs(true);
            else new AlertDialog.Builder(app).setTitle(text("Disable Logs"))
                .setMessage(text("Stop recording session logs?")).setNegativeButton(text("Cancel"), null)
                .setPositiveButton(text("Disable Logs"), (d, w) -> saveLogs(false)).show();
        }));
    }

    private void saveLogs(boolean enabled) {
        JSONObject response = app.call("logsEnabled", "enabled", enabled);
        if (response.has("error")) app.toast(response.optString("error"));
        else app.renderSettings();
    }

    private void terminal(LinearLayout page) {
        JSONObject state = app.call("terminalSettings");
        LinearLayout general = card(page, "Terminal settings");
        if (state.has("error")) {
            description(general, state.optString("error"));
            return;
        }
        toggle(general, "Right click to paste", "rightClickPaste", state);
        toggle(general, "Bell sound", "bellSound", state);
        toggle(general, "Use bright colours for bold text", "brightenBold", state);
        JSONArray emulations = state.optJSONArray("emulations");
        if (emulations != null) {
            String[] values = new String[emulations.length()];
            int selected = 0;
            for (int i = 0; i < values.length; i++) {
                values[i] = emulations.optString(i);
                if (values[i].equals(state.optString("emulation"))) selected = i;
            }
            final int index = selected;
            description(general, "Terminal emulation type");
            action(general, values[index], () -> choose("Terminal emulation type", values, index, value -> {
                if (saveTerminal("emulation", values[value])) app.renderSettings();
            }));
        }

        LinearLayout font = card(page, "Text Size");
        number(font, "Text Size", "font", state.optLong("font"), state.optLong("minFont"), state.optLong("maxFont"));

        LinearLayout theme = card(page, "Terminal theme");
        theme.addView(app.themes().new Choice(state.optString("theme"), null, name -> saveTerminal("theme", name)));

        LinearLayout keepalive = card(page, "Keepalive Interval");
        description(keepalive, "How often (in seconds) to send SSH-level keepalive packets to the server. Set to 0 to disable.");
        number(keepalive, "Interval", "keepAlive", state.optLong("keepAlive"), 0, Integer.MAX_VALUE);

        LinearLayout scrollback = card(page, "Scrollback");
        description(scrollback, "Limit number of terminal rows. Set to 0 to maximum limit size.");
        number(scrollback, "Number of rows", "scrollback", state.optLong("scrollback"), 0, 4294967295L);

        LinearLayout pqc = card(page, "Post-Quantum Key Exchange");
        toggle(pqc, "Post-Quantum Key Exchange", "pqc", state);
        description(pqc, "Turn off if you're experiencing issues with legacy devices");
    }

    private boolean saveTerminal(String field, Object value) {
        JSONObject result = app.call("saveTerminal", "field", field, "value", value);
        if (result.has("error")) { app.toast(result.optString("error")); return false; }
        app.applyTerminalSettings(result);
        return true;
    }

    private void number(LinearLayout card, String label, String field, long current, long min, long max) {
        action(card, text(label) + ": " + current, () -> {
            LinearLayout form = app.column();
            EditText input = app.field(form, text(label), Long.toString(current), InputType.TYPE_CLASS_NUMBER);
            AlertDialog dialog = new AlertDialog.Builder(app).setTitle(text(label)).setView(app.scroll(form))
                .setNegativeButton(text("Cancel"), null).setPositiveButton(text("Save"), null).show();
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(view -> {
                long value;
                try { value = Long.parseLong(MainActivity.value(input).trim()); }
                catch (NumberFormatException e) { value = -1; }
                if (value < min || value > max) {
                    input.setError(min + "–" + max);
                    return;
                }
                if (saveTerminal(field, value)) {
                    dialog.dismiss();
                    app.renderSettings();
                }
            });
        });
    }

    private void toggle(LinearLayout card, String label, String field, JSONObject state) {
        Switch toggle = new Switch(app);
        toggle.setText(text(label));
        toggle.setMinHeight(app.dp(48));
        toggle.setChecked(state.optBoolean(field));
        card.addView(toggle, new LinearLayout.LayoutParams(-1, -2));
        toggle.setOnCheckedChangeListener((button, value) -> {
            if (!saveTerminal(field, value)) app.renderSettings();
        });
    }

    private void choose(String title, String[] values, int selected, java.util.function.IntConsumer choice) {
        String[] labels = java.util.Arrays.stream(values).map(MainActivity::text).toArray(String[]::new);
        new AlertDialog.Builder(app).setTitle(text(title)).setSingleChoiceItems(labels, selected, (d, i) -> {
            d.dismiss();
            choice.accept(i);
        }).setNegativeButton(text("Cancel"), null).show();
    }

    private void language(LinearLayout page) {
        LinearLayout card = card(page, "Language");
        boolean russian = preferences.getBoolean("ui:russian", false);
        card.addView(app.style.section("check", "English", !russian, () -> app.setLanguage(false)),
            new LinearLayout.LayoutParams(-1, app.dp(48)));
        card.addView(app.style.section("check", "Русский", russian, () -> app.setLanguage(true)),
            new LinearLayout.LayoutParams(-1, app.dp(48)));
    }

    private boolean shortcutsEnabled() { return preferences.getBoolean("ui:shortcuts", true); }

    private void shortcuts(LinearLayout page) {
        LinearLayout card = card(page, "Shortcuts");
        String[] presets = {"PC hotkeys", "Disabled"};
        action(card, presets[shortcutsEnabled() ? 0 : 1], () -> choose("Shortcuts", presets, shortcutsEnabled() ? 0 : 1, index -> {
            preferences.edit().putBoolean("ui:shortcuts", index == 0).apply();
            app.renderSettings();
        }));
        if (shortcutsEnabled()) for (int i = 0; i < shortcuts.length(); i++) {
            JSONObject binding = shortcuts.optJSONObject(i);
            String keys = binding.optString("keys").replace("ctrl", "Ctrl").replace("shift", "Shift").replace("alt", "Alt");
            card.addView(app.label(text(binding.optString("title")) + "  " + keys, 14, TermianStyle.TEXT));
        }
    }

    String shortcut(KeyEvent event) {
        if (!shortcutsEnabled() || event.isMetaPressed() || (!event.isCtrlPressed() && !event.isAltPressed())) return "";
        int code = event.getKeyCode();
        String key;
        if (code >= KeyEvent.KEYCODE_A && code <= KeyEvent.KEYCODE_Z) key = String.valueOf((char) ('a' + code - KeyEvent.KEYCODE_A));
        else if (code == KeyEvent.KEYCODE_DPAD_LEFT) key = "←";
        else if (code == KeyEvent.KEYCODE_DPAD_RIGHT) key = "→";
        else return "";
        String chord = (event.isCtrlPressed() ? "ctrl+" : "") + (event.isAltPressed() ? "alt+" : "")
            + (event.isShiftPressed() ? "shift+" : "") + key;
        for (int i = 0; i < shortcuts.length(); i++) {
            JSONObject binding = shortcuts.optJSONObject(i);
            if (chord.equals(binding.optString("keys"))) return binding.optString("id");
        }
        return "";
    }
}
