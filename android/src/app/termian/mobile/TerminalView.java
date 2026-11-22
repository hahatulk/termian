package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.text.InputType;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewConfiguration;
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
    private final int padding, touchSlop;
    private String tabId = "";
    private boolean secondary;
    private float downX, downY, lastY, wheelX, wheelY;
    private boolean scrolled;
    private int mouseButtons, reportedButtons;

    TerminalView(MainActivity activity) {
        super(activity);
        this.activity = activity;
        setFocusable(true);
        setFocusableInTouchMode(true);
        setContentDescription(text("Терминал SSH"));
        padding = activity.dp(6);
        touchSlop = ViewConfiguration.get(activity).getScaledTouchSlop();
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
        mouseButtons = reportedButtons = 0;
        wheelX = wheelY = 0;
    }

    void tag(String id, boolean secondary) {
        tabId = id;
        this.secondary = secondary;
        setContentDescription(secondary ? "Second terminal pane" : text("Терминал SSH"));
    }

    void reportSize() {
        if (getWidth() <= 2 * padding || getHeight() <= 2 * padding) return;
        activity.call("size", "id", tabId, "columns", columns(), "rows", rows());
    }

    private int columns() { return Math.max(2, Math.min(256, (int) ((getWidth() - 2 * padding) / cellWidth))); }

    private int rows() { return Math.max(2, Math.min(160, (int) ((getHeight() - 2 * padding) / lineHeight))); }

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

    private boolean mouseTracking() {
        return frame != null && frame.optBoolean("mouseTracking");
    }

    private boolean localPointer(MotionEvent event) {
        return !mouseTracking() || (event.getMetaState() & KeyEvent.META_SHIFT_MASK) != 0;
    }

    private boolean reportMouse(String kind, int button, MotionEvent event, int dx, int dy, boolean clamp) {
        if (tabId.isEmpty() || activity.privacyLocked()) return false;
        float x = event.getX() - padding, y = event.getY() - padding;
        if (!clamp && (x < 0 || y < 0 || x >= columns() * cellWidth || y >= rows() * lineHeight
            || event.getX() >= getWidth() - padding || event.getY() >= getHeight() - padding)) return false;
        int column = Math.max(0, Math.min(columns() - 1, (int) (x / cellWidth)));
        int row = Math.max(0, Math.min(rows() - 1, (int) (y / lineHeight)));
        int mods = event.getMetaState();
        return activity.call("mouse", "id", tabId, "kind", kind, "button", button, "row", row,
            "column", column, "dx", dx, "dy", dy, "ctrl", (mods & KeyEvent.META_CTRL_MASK) != 0,
            "alt", (mods & KeyEvent.META_ALT_MASK) != 0, "shift", (mods & KeyEvent.META_SHIFT_MASK) != 0)
            .optBoolean("handled");
    }

    private void scroll(MotionEvent event, int columns, int rows, boolean clamp) {
        if (columns == 0 && rows == 0) return;
        if (!reportMouse("scroll", -1, event, columns, rows, clamp) && rows != 0
            && !activity.privacyLocked()) {
            activity.call("scroll", "id", tabId, "lines", rows);
        }
    }

    private boolean fingerMoved(MotionEvent event) {
        return Math.abs(event.getX() - downX) > touchSlop || Math.abs(event.getY() - downY) > touchSlop;
    }

    @Override public boolean onTouchEvent(MotionEvent event) {
        if (event.isFromSource(InputDevice.SOURCE_MOUSE)) return mouseTouch(event);
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                downX = event.getX();
                downY = lastY = event.getY();
                scrolled = false;
                return true;
            case MotionEvent.ACTION_POINTER_DOWN:
                scrolled = true;
                return true;
            case MotionEvent.ACTION_POINTER_UP:
                lastY = event.getY(event.getActionIndex() == 0 ? 1 : 0);
                return true;
            case MotionEvent.ACTION_MOVE:
                scrolled |= fingerMoved(event);
                if (event.getPointerCount() != 1) return true;
                int lines = (int) ((event.getY() - lastY) / lineHeight);
                if (lines != 0) {
                    scrolled = true;
                    lastY += lines * lineHeight;
                    scroll(event, 0, lines, true);
                }
                return true;
            case MotionEvent.ACTION_UP:
                if (!scrolled && !fingerMoved(event) && !reportMouse("tap", 0, event, 0, 0, false)
                    && localPointer(event)) performClick();
                return true;
            case MotionEvent.ACTION_CANCEL:
                return true;
            default:
                return super.onTouchEvent(event);
        }
    }

    private static int buttonCode(int button) {
        switch (button) {
            case MotionEvent.BUTTON_PRIMARY: return 0;
            case MotionEvent.BUTTON_TERTIARY: return 1;
            case MotionEvent.BUTTON_SECONDARY: return 2;
            default: return -1;
        }
    }

    private void mouseDown(MotionEvent event, int button) {
        if ((mouseButtons & button) != 0) return;
        mouseButtons |= button;
        if (reportMouse("press", buttonCode(button), event, 0, 0, false)) reportedButtons |= button;
        else if (button == MotionEvent.BUTTON_SECONDARY && !activity.privacyLocked()
            && localPointer(event)) activity.pasteByMouse(tabId);
    }

    private void mouseUp(MotionEvent event, int button, boolean click) {
        if ((mouseButtons & button) == 0) return;
        mouseButtons &= ~button;
        if ((reportedButtons & button) != 0) {
            reportedButtons &= ~button;
            reportMouse("release", buttonCode(button), event, 0, 0, true);
        } else if (click && button == MotionEvent.BUTTON_PRIMARY && localPointer(event)) performClick();
    }

    private boolean mouseTouch(MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                if (event.isButtonPressed(MotionEvent.BUTTON_PRIMARY)) mouseDown(event, MotionEvent.BUTTON_PRIMARY);
                return true;
            case MotionEvent.ACTION_MOVE:
                int dragging = Integer.lowestOneBit(event.getButtonState() & reportedButtons);
                if (dragging != 0) reportMouse("move", buttonCode(dragging), event, 0, 0, true);
                return true;
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_CANCEL:
                boolean click = event.getActionMasked() == MotionEvent.ACTION_UP;
                for (int remaining = mouseButtons; remaining != 0; remaining &= remaining - 1) {
                    mouseUp(event, Integer.lowestOneBit(remaining), click);
                }
                return true;
            default:
                return super.onTouchEvent(event);
        }
    }

    @Override public boolean performClick() {
        super.performClick();
        if (activity.privacyLocked()) return true;
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
                activity.typed(tabId, text.toString());
                return true;
            }

            @Override public boolean deleteSurroundingText(int before, int after) {
                for (int i = 0; i < Math.min(before, 32); i++) activity.key(tabId, "backspace", null, false, false, false);
                for (int i = 0; i < Math.min(after, 32); i++) activity.key(tabId, "delete", null, false, false, false);
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

    @Override public boolean onHoverEvent(MotionEvent event) {
        if (event.isFromSource(InputDevice.SOURCE_MOUSE) && mouseTracking()
            && (event.getActionMasked() == MotionEvent.ACTION_HOVER_MOVE || event.getActionMasked() == MotionEvent.ACTION_HOVER_ENTER)
            && reportMouse("move", -1, event, 0, 0, false)) return true;
        return super.onHoverEvent(event);
    }

    @Override public boolean onGenericMotionEvent(MotionEvent event) {
        if (event.isFromSource(InputDevice.SOURCE_CLASS_POINTER) && event.getActionMasked() == MotionEvent.ACTION_SCROLL) {
            wheelX -= event.getAxisValue(MotionEvent.AXIS_HSCROLL);
            wheelY += event.getAxisValue(MotionEvent.AXIS_VSCROLL);
            int columns = (int) wheelX, rows = (int) wheelY;
            wheelX -= columns;
            wheelY -= rows;
            scroll(event, columns, rows, false);
            return true;
        }
        int button = event.getActionButton();
        if (buttonCode(button) >= 0) {
            if (event.getActionMasked() == MotionEvent.ACTION_BUTTON_PRESS) {
                mouseDown(event, button);
                return true;
            }
            if (event.getActionMasked() == MotionEvent.ACTION_BUTTON_RELEASE) {
                mouseUp(event, button, true);
                return true;
            }
        }
        return super.onGenericMotionEvent(event);
    }

    private boolean sendKey(String key, String typed, KeyEvent event) {
        activity.key(tabId, key, typed, event.isCtrlPressed(), event.isAltPressed(), event.isShiftPressed());
        return true;
    }

    @Override public boolean onKeyDown(int code, KeyEvent event) {
        if (event.isMetaPressed()) return super.onKeyDown(code, event);
        if (code >= KeyEvent.KEYCODE_F1 && code <= KeyEvent.KEYCODE_F12)
            return sendKey("f" + (code - KeyEvent.KEYCODE_F1 + 1), null, event);
        if (code >= KeyEvent.KEYCODE_NUMPAD_0 && code <= KeyEvent.KEYCODE_NUMPAD_9)
            return sendKey("numpad" + (code - KeyEvent.KEYCODE_NUMPAD_0), null, event);
        String key;
        switch (code) {
            case KeyEvent.KEYCODE_ENTER: key = "enter"; break;
            case KeyEvent.KEYCODE_DEL: key = "backspace"; break;
            case KeyEvent.KEYCODE_FORWARD_DEL: key = "delete"; break;
            case KeyEvent.KEYCODE_TAB: key = "tab"; break;
            case KeyEvent.KEYCODE_ESCAPE: key = "escape"; break;
            case KeyEvent.KEYCODE_SPACE: key = "space"; break;
            case KeyEvent.KEYCODE_DPAD_UP: key = "up"; break;
            case KeyEvent.KEYCODE_DPAD_DOWN: key = "down"; break;
            case KeyEvent.KEYCODE_DPAD_LEFT: key = "left"; break;
            case KeyEvent.KEYCODE_DPAD_RIGHT: key = "right"; break;
            case KeyEvent.KEYCODE_MOVE_HOME: key = "home"; break;
            case KeyEvent.KEYCODE_MOVE_END: key = "end"; break;
            case KeyEvent.KEYCODE_PAGE_UP: key = "pageup"; break;
            case KeyEvent.KEYCODE_PAGE_DOWN: key = "pagedown"; break;
            case KeyEvent.KEYCODE_INSERT: key = "insert"; break;
            case KeyEvent.KEYCODE_NUMPAD_DIVIDE: key = "numpaddivide"; break;
            case KeyEvent.KEYCODE_NUMPAD_MULTIPLY: key = "numpadmultiply"; break;
            case KeyEvent.KEYCODE_NUMPAD_SUBTRACT: key = "numpadsubtract"; break;
            case KeyEvent.KEYCODE_NUMPAD_ADD: key = "numpadadd"; break;
            case KeyEvent.KEYCODE_NUMPAD_DOT: key = "numpaddot"; break;
            case KeyEvent.KEYCODE_NUMPAD_COMMA: key = "numpadcomma"; break;
            case KeyEvent.KEYCODE_NUMPAD_ENTER: key = "numpadenter"; break;
            case KeyEvent.KEYCODE_NUMPAD_EQUALS: key = "numpadequals"; break;
            default:
                int modifiers = event.getMetaState() & ~(KeyEvent.META_CTRL_MASK | KeyEvent.META_ALT_MASK);
                int typed = event.getUnicodeChar(modifiers);
                if ((event.getMetaState() & KeyEvent.META_ALT_RIGHT_ON) != 0) {
                    int composed = event.getUnicodeChar();
                    if (composed > 0 && composed != typed && Character.isValidCodePoint(composed)
                        && !Character.isISOControl(composed)) {
                        activity.typed(tabId, new String(Character.toChars(composed)));
                        return true;
                    }
                }
                if (typed <= 0 || !Character.isValidCodePoint(typed)) return super.onKeyDown(code, event);
                String text = new String(Character.toChars(typed));
                return sendKey(text, text, event);
        }
        return sendKey(key, null, event);
    }
}
