package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import org.json.JSONArray;
import org.json.JSONObject;

final class TerminalView extends View {
    private final MainActivity activity;
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Typeface regular = Typeface.MONOSPACE;
    private final Typeface bold = Typeface.create(Typeface.MONOSPACE, Typeface.BOLD);
    private JSONObject frame;
    private float cellWidth, lineHeight, baseline;
    private final int padding;
    private String tabId = "";
    private boolean secondary;
    private float lastY;
    private boolean scrolled;

    TerminalView(MainActivity activity) {
        super(activity);
        this.activity = activity;
        setFocusable(true);
        setFocusableInTouchMode(true);
        setContentDescription(text("Терминал SSH"));
        padding = activity.dp(6);
        font(14);
    }

    void font(int size) {
        paint.setTypeface(regular);
        paint.setTextSize(activity.dp(size));
        cellWidth = (float) Math.ceil(paint.measureText("M"));
        Paint.FontMetrics font = paint.getFontMetrics();
        lineHeight = (float) Math.ceil(font.descent - font.ascent);
        baseline = -font.ascent;
        reportSize();
        invalidate();
    }

    void frame(JSONObject frame) {
        this.frame = frame;
        invalidate();
    }

    void clear() {
        frame(null);
    }

    void tag(String id, boolean secondary) {
        tabId = id;
        this.secondary = secondary;
        setContentDescription(secondary ? "Second terminal pane" : text("Терминал SSH"));
    }

    void reportSize() {
        if (getWidth() <= 2 * padding || getHeight() <= 2 * padding) return;
        activity.call("size", "id", tabId, "columns", (int) ((getWidth() - 2 * padding) / cellWidth),
            "rows", (int) ((getHeight() - 2 * padding) / lineHeight));
    }

    @Override protected void onSizeChanged(int w, int h, int oldW, int oldH) {
        reportSize();
    }

    @Override public void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        canvas.drawColor(frame == null ? 0xff171b20 : opaque(frame.optInt("bg")));
        if (frame == null) return;
        JSONArray lines = frame.optJSONArray("lines");
        if (lines == null) return;
        canvas.save();
        canvas.clipRect(padding, padding, getWidth() - padding, getHeight() - padding);
        for (int r = 0; r < lines.length(); r++) {
            float x = padding;
            float y = padding + r * lineHeight;
            JSONArray row = lines.optJSONArray(r);
            for (int j = 0; row != null && j < row.length(); j++) {
                JSONArray run = row.optJSONArray(j);
                if (run == null) continue;
                float end = x + run.optInt(1) * cellWidth;
                paint.setColor(opaque(run.optInt(3)));
                canvas.drawRect(x, y, end, y + lineHeight, paint);
                String text = run.optString(0);
                int endOfText = text.length();
                while (endOfText > 0 && text.charAt(endOfText - 1) == ' ') endOfText--;
                text = text.substring(0, endOfText);
                if (!text.isEmpty()) {
                    paint.setColor(opaque(run.optInt(2)));
                    paint.setTypeface(run.optBoolean(4) ? bold : regular);
                    canvas.drawText(text, x, y + baseline, paint);
                }
                x = end;
            }
        }
        JSONArray cursor = frame.optJSONArray("cursor");
        if (cursor != null) {
            float y = padding + (cursor.optInt(0) + 1) * lineHeight;
            float x = padding + cursor.optInt(1) * cellWidth;
            paint.setColor(opaque(frame.optInt("cursorColor")));
            canvas.drawRect(x, y - activity.dp(2), x + cellWidth, y, paint);
        }
        canvas.restore();
    }

    private int opaque(int rgb) { return 0xff000000 | rgb; }

    @Override public boolean onTouchEvent(MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                lastY = event.getY();
                scrolled = false;
                return true;
            case MotionEvent.ACTION_MOVE:
                int lines = (int) ((event.getY() - lastY) / lineHeight);
                if (lines != 0) {
                    scrolled = true;
                    lastY += lines * lineHeight;
                    activity.call("scroll", "id", tabId, "lines", lines);
                }
                return true;
            case MotionEvent.ACTION_UP:
                if (!scrolled) performClick();
                return true;
            case MotionEvent.ACTION_CANCEL:
                return true;
            default:
                return super.onTouchEvent(event);
        }
    }

    @Override public boolean performClick() {
        super.performClick();
        if (secondary) activity.focusPane(tabId);
        else activity.keyboard();
        return true;
    }

    @Override public boolean onCheckIsTextEditor() { return true; }

    @Override public InputConnection onCreateInputConnection(EditorInfo info) {
        info.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS |
            InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD;
        info.imeOptions = EditorInfo.IME_ACTION_NONE | EditorInfo.IME_FLAG_NO_EXTRACT_UI |
            EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING;
        info.initialSelStart = info.initialSelEnd = 0;
        return new BaseInputConnection(this, false) {
            @Override public boolean commitText(CharSequence text, int cursor) {
                activity.typed(text.toString());
                return true;
            }

            @Override public boolean deleteSurroundingText(int before, int after) {
                for (int i = 0; i < Math.min(before, 32); i++) activity.key("backspace", null, false, false, false);
                for (int i = 0; i < Math.min(after, 32); i++) activity.key("delete", null, false, false, false);
                return true;
            }

            @Override public boolean deleteSurroundingTextInCodePoints(int before, int after) {
                return deleteSurroundingText(before, after);
            }

            @Override public boolean sendKeyEvent(KeyEvent event) {
                return event.getAction() != KeyEvent.ACTION_DOWN || TerminalView.this.onKeyDown(event.getKeyCode(), event);
            }
        };
    }

    @Override public boolean onKeyDown(int code, KeyEvent event) {
        String key;
        switch (code) {
            case KeyEvent.KEYCODE_ENTER: key = "enter"; break;
            case KeyEvent.KEYCODE_DEL: key = "backspace"; break;
            case KeyEvent.KEYCODE_FORWARD_DEL: key = "delete"; break;
            case KeyEvent.KEYCODE_TAB: key = "tab"; break;
            case KeyEvent.KEYCODE_ESCAPE: key = "escape"; break;
            case KeyEvent.KEYCODE_DPAD_UP: key = "up"; break;
            case KeyEvent.KEYCODE_DPAD_DOWN: key = "down"; break;
            case KeyEvent.KEYCODE_DPAD_LEFT: key = "left"; break;
            case KeyEvent.KEYCODE_DPAD_RIGHT: key = "right"; break;
            case KeyEvent.KEYCODE_MOVE_HOME: key = "home"; break;
            case KeyEvent.KEYCODE_MOVE_END: key = "end"; break;
            case KeyEvent.KEYCODE_PAGE_UP: key = "pageup"; break;
            case KeyEvent.KEYCODE_PAGE_DOWN: key = "pagedown"; break;
            default:
                int modifiers = event.getMetaState() & ~(KeyEvent.META_CTRL_MASK | KeyEvent.META_ALT_MASK);
                int typed = event.getUnicodeChar(modifiers);
                if (typed <= 0 || !Character.isValidCodePoint(typed)) return super.onKeyDown(code, event);
                String text = new String(Character.toChars(typed));
                activity.key(" ".equals(text) ? "space" : text, text, event.isCtrlPressed(), event.isAltPressed(), event.isShiftPressed());
                return true;
        }
        activity.key(key, null, event.isCtrlPressed(), event.isAltPressed(), event.isShiftPressed());
        return true;
    }
}
