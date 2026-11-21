package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.app.AlertDialog;
import android.graphics.Typeface;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.BaseAdapter;
import android.widget.LinearLayout;
import android.widget.ListView;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.function.Predicate;
import org.json.JSONArray;
import org.json.JSONObject;

final class MobileThemes {
    private final MainActivity app;
    private final ArrayList<JSONObject> themes = new ArrayList<>();

    MobileThemes(MainActivity app) {
        this.app = app;
        JSONArray values = app.call("terminalThemes").optJSONArray("themes");
        if (values != null) for (int i = 0; i < values.length(); i++) {
            JSONObject theme = values.optJSONObject(i);
            if (theme != null) themes.add(theme);
        }
    }

    private JSONObject find(String name) {
        for (JSONObject theme : themes) if (theme.optString("name").equalsIgnoreCase(name)) return theme;
        return null;
    }

    private String title(String name, String global) {
        return name.isEmpty() && global != null ? text("Default ({global})").replace("{global}", global) : name;
    }

    private View row(String name, String global, boolean selected, boolean picker) {
        LinearLayout row = new LinearLayout(app);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(app.dp(10), app.dp(7), app.dp(10), app.dp(7));
        row.setMinimumHeight(app.dp(58));
        row.setSelected(selected);
        JSONObject theme = find(name.isEmpty() ? global : name);
        if (theme != null) {
            LinearLayout sample = app.column();
            sample.setGravity(Gravity.CENTER);
            sample.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS);
            sample.setBackground(app.style.shape(0xff000000 | theme.optInt("background"), 6));
            TextView prompt = app.style.text("Aa", 13, 0xff000000 | theme.optInt("foreground"), 500);
            prompt.setTypeface(Typeface.MONOSPACE);
            sample.addView(prompt);
            LinearLayout colors = new LinearLayout(app);
            colors.setGravity(Gravity.CENTER);
            JSONArray swatches = theme.optJSONArray("colors");
            if (swatches != null) for (int i = 0; i < swatches.length(); i++) {
                View swatch = new View(app);
                swatch.setBackground(app.style.shape(0xff000000 | swatches.optInt(i), 4));
                LinearLayout.LayoutParams size = new LinearLayout.LayoutParams(app.dp(7), app.dp(7));
                size.setMargins(app.dp(2), app.dp(3), app.dp(2), 0);
                colors.addView(swatch, size);
            }
            sample.addView(colors);
            row.addView(sample, new LinearLayout.LayoutParams(app.dp(80), app.dp(44)));
        }
        TextView nameView = app.style.text(title(name, global), 14, TermianStyle.TEXT, selected ? 700 : 500);
        nameView.setPadding(app.dp(10), 0, app.dp(6), 0);
        row.addView(nameView, new LinearLayout.LayoutParams(0, -2, 1));
        if (!picker || selected) row.addView(
            app.style.icon(picker ? "check" : "caret_down", picker ? TermianStyle.GREEN_TEXT : TermianStyle.MUTED),
            new LinearLayout.LayoutParams(app.dp(16), app.dp(16)));
        app.style.action(row, selected ? TermianStyle.ACTIVE : TermianStyle.SIDEBAR);
        row.setFocusable(!picker);
        return row;
    }

    final class Choice extends LinearLayout {
        private String name;
        private final String global;
        private final Predicate<String> save;

        Choice(String name, String global, Predicate<String> save) {
            super(app);
            this.name = name;
            this.global = global;
            this.save = save;
            redraw();
        }

        String value() { return name; }

        private void redraw() {
            removeAllViews();
            View current = row(name, global, false, false);
            current.setContentDescription(text("Terminal theme") + ": " + title(name, global));
            current.setOnClickListener(view -> open());
            addView(current, new LinearLayout.LayoutParams(-1, -2));
        }

        private void open() {
            ArrayList<String> options = new ArrayList<>();
            if (global != null) options.add("");
            for (JSONObject theme : themes) options.add(theme.optString("name"));
            ListView list = new ListView(app);
            list.setDivider(null);
            list.setAdapter(new BaseAdapter() {
                @Override public int getCount() { return options.size(); }
                @Override public String getItem(int position) { return options.get(position); }
                @Override public long getItemId(int position) { return position; }
                @Override public View getView(int position, View convertView, ViewGroup parent) {
                    String option = getItem(position);
                    return row(option, global, option.equalsIgnoreCase(name), true);
                }
            });
            AlertDialog dialog = new AlertDialog.Builder(app).setTitle(text("Select Color Theme")).setView(list)
                .setNegativeButton(text("Cancel"), null).create();
            list.setOnItemClickListener((parent, view, position, id) -> {
                String selected = options.get(position);
                if (selected.equalsIgnoreCase(name) || save.test(selected)) {
                    name = selected;
                    redraw();
                    dialog.dismiss();
                }
            });
            dialog.show();
            list.setSelection(Math.max(0, options.indexOf(name)));
        }
    }
}
