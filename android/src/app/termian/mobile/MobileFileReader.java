package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.graphics.Typeface;
import android.text.Layout;
import android.text.TextUtils;
import android.text.format.Formatter;
import android.view.Gravity;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.ScrollView;
import android.widget.TextView;
import java.util.ArrayDeque;
import org.json.JSONObject;

final class MobileFileReader {
    final LinearLayout view;
    private final MainActivity app;
    private final String name;
    private final TextView content, location, pageNumber, notice;
    private final ScrollView scroll;
    private final ProgressBar progress;
    private final View previous, next;
    private final ArrayDeque<Long> back = new ArrayDeque<>();
    private String request;
    private long start, requestedStart, nextStart = -1;
    private boolean loaded, connected = true;

    MobileFileReader(MainActivity app, String name, String request, Runnable close) {
        this.app = app;
        this.name = name;
        this.request = request;
        view = app.column();
        view.setBackgroundColor(TermianStyle.BG);

        LinearLayout header = new LinearLayout(app);
        header.setGravity(Gravity.CENTER_VERTICAL);
        header.setPadding(app.dp(4), app.dp(4), app.dp(12), app.dp(4));
        header.setBackgroundColor(TermianStyle.SIDEBAR);
        header.addView(app.style.iconAction("sftp_arrow", text("Back"), close));
        LinearLayout title = app.column();
        TextView caption = app.style.text(text("File preview"), 12, TermianStyle.GREEN_TEXT, 500);
        title.addView(caption);
        TextView fileName = app.style.text(name, 16, TermianStyle.TEXT, 700);
        fileName.setSingleLine(true);
        fileName.setEllipsize(TextUtils.TruncateAt.END);
        title.addView(fileName);
        header.addView(title, new LinearLayout.LayoutParams(0, -2, 1));
        view.addView(header, new LinearLayout.LayoutParams(-1, -2));

        progress = new ProgressBar(app, null, android.R.attr.progressBarStyleHorizontal);
        progress.setIndeterminate(true);
        view.addView(progress, new LinearLayout.LayoutParams(-1, app.dp(2)));
        notice = app.style.text("", 13, TermianStyle.ERROR, 500);
        notice.setPadding(app.dp(16), app.dp(8), app.dp(16), app.dp(8));
        notice.setVisibility(View.GONE);
        notice.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        view.addView(notice, new LinearLayout.LayoutParams(-1, -2));

        scroll = new ScrollView(app);
        scroll.setFillViewport(true);
        content = app.style.text("", 13, TermianStyle.TEXT, 400);
        content.setTypeface(Typeface.MONOSPACE);
        content.setLineSpacing(app.dp(3), 1f);
        content.setBreakStrategy(Layout.BREAK_STRATEGY_SIMPLE);
        content.setHyphenationFrequency(Layout.HYPHENATION_FREQUENCY_NONE);
        content.setPadding(app.dp(16), app.dp(12), app.dp(16), app.dp(12));
        content.setTextIsSelectable(true);
        content.setSaveEnabled(false);
        scroll.addView(content, new android.widget.FrameLayout.LayoutParams(-1, -2));
        view.addView(scroll, new LinearLayout.LayoutParams(-1, 0, 1));

        LinearLayout footer = new LinearLayout(app);
        footer.setGravity(Gravity.CENTER_VERTICAL);
        footer.setPadding(app.dp(10), app.dp(4), app.dp(10), app.dp(4));
        footer.setBackgroundColor(TermianStyle.SIDEBAR);
        LinearLayout info = app.column();
        info.setPadding(app.dp(6), 0, app.dp(8), 0);
        pageNumber = app.style.text("", 14, TermianStyle.TEXT, 500);
        pageNumber.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        info.addView(pageNumber);
        location = app.style.text("", 12, TermianStyle.MUTED, 400);
        info.addView(location);
        footer.addView(info, new LinearLayout.LayoutParams(0, -2, 1));
        previous = app.style.iconAction("sftp_arrow", text("Previous page"), () -> {
            if (!back.isEmpty()) load(back.peek());
        });
        next = app.style.iconAction("sftp_arrow", text("Next page"), TermianStyle.GREEN_TEXT, 48, 18, () -> {
            if (nextStart >= 0) load(nextStart);
        });
        next.setRotation(180);
        footer.addView(previous);
        footer.addView(next);
        view.addView(footer, new LinearLayout.LayoutParams(-1, -2));
        buttonState();
    }

    private void load(long offset) {
        if (!connected || !request.isEmpty()) return;
        JSONObject response = app.call("filesPreview", "name", name, "offset", offset);
        if (response.has("error")) {
            showError(response.optString("error"));
            return;
        }
        requestedStart = offset;
        request = response.optString("request");
        notice.setVisibility(View.GONE);
        buttonState();
    }

    void receive(JSONObject response) {
        if (request.isEmpty() || !request.equals(response.optString("request"))) return;
        request = "";
        if (response.has("error")) {
            showError(response.optString("error"));
        } else {
            if (loaded && requestedStart > start) back.push(start);
            else if (loaded && requestedStart < start && !back.isEmpty()) back.pop();
            start = requestedStart;
            loaded = true;
            nextStart = response.optLong("next", -1);
            String page = response.optString("text");
            content.setText(page.isEmpty() ? text("File is empty.") : page);
            scroll.scrollTo(0, 0);
            scroll.post(() -> scroll.scrollTo(0, 0));
            notice.setVisibility(View.GONE);
            long end = response.optLong("end");
            long size = response.optLong("size", -1);
            String read = Formatter.formatShortFileSize(app, end);
            location.setText(size < 0 ? read : read + " / " + Formatter.formatShortFileSize(app, size));
        }
        buttonState();
    }

    void disconnected(String message) {
        connected = false;
        request = "";
        showError(message);
        buttonState();
    }

    private void showError(String message) {
        notice.setText(text(message));
        notice.setVisibility(View.VISIBLE);
    }

    private void buttonState() {
        boolean reading = !request.isEmpty();
        progress.setVisibility(reading ? View.VISIBLE : View.INVISIBLE);
        pageNumber.setText(reading ? text("Reading file…") : loaded
            ? text("Page {number}").replace("{number}", Integer.toString(back.size() + 1))
            : text("File preview"));
        enable(previous, connected && !reading && !back.isEmpty());
        enable(next, connected && !reading && nextStart >= 0);
    }

    private void enable(View button, boolean enabled) {
        button.setEnabled(enabled);
        button.setAlpha(enabled ? 1f : .4f);
    }
}
