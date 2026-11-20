package app.termian.mobile;

import android.content.res.ColorStateList;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
import android.view.Gravity;
import android.view.View;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

final class TermianStyle {
    static final int BAR = 0xff151826;
    static final int BG = 0xff1e2133;
    static final int SIDEBAR = 0xff23263a;
    static final int SURFACE = 0xff292c3f;
    static final int ACTIVE = 0xff353950;
    static final int INPUT = 0xff3a3e54;
    static final int TEXT = 0xffe6e8ef;
    static final int MUTED = 0xff8d91a5;
    static final int GREEN = 0xff21b568;
    static final int GREEN_TEXT = 0xff71d99c;
    static final int CONNECTING = 0xff1f3b2f;
    static final int CONNECTING_TEXT = 0xff3dbd6e;
    static final int TILE = 0xff0c4a79;
    static final int ERROR = 0xfff25e61;
    private final MainActivity context;
    private final Typeface book, medium, bold;

    TermianStyle(MainActivity context) {
        this.context = context;
        book = load("Book");
        medium = load("Medium");
        bold = load("Bold");
    }

    private Typeface load(String name) {
        return Typeface.createFromAsset(context.getAssets(), "fonts/CircularXX-" + name + ".ttf");
    }

    TextView text(CharSequence content, float sp, int color, int weight) {
        TextView text = new TextView(context);
        text.setText(content);
        text.setTextSize(sp);
        text.setTextColor(color);
        text.setTypeface(weight >= 700 ? bold : weight >= 500 ? medium : book);
        return text;
    }

    GradientDrawable shape(int color, float radius) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(color);
        drawable.setCornerRadius(context.dp(radius));
        return drawable;
    }

    void action(View view, int color) {
        action(view,color,8);
    }

    void action(View view, int color, int radius) {
        view.setBackground(new RippleDrawable(ColorStateList.valueOf(0x28ffffff), shape(color, radius), shape(0xffffffff, radius)));
        view.setFocusable(true);
    }

    ImageView icon(String name, int color) {
        ImageView icon = new ImageView(context);
        int id = context.getResources().getIdentifier("term_" + name, "drawable", context.getPackageName());
        if (id != 0) icon.setImageResource(id);
        icon.setColorFilter(color);
        icon.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        return icon;
    }

    View iconAction(String name, String label, Runnable click) {
        return iconAction(name,label,TEXT,48,18,click);
    }

    View iconAction(String name, String label, int color, int touchSize, int iconSize, Runnable click) {
        LinearLayout container = new LinearLayout(context);
        container.setGravity(Gravity.CENTER);
        container.setMinimumWidth(context.dp(touchSize));
        container.setMinimumHeight(context.dp(touchSize));
        container.setContentDescription(label);
        container.addView(icon(name,color), new LinearLayout.LayoutParams(context.dp(iconSize), context.dp(iconSize)));
        container.setOnClickListener(v -> click.run());
        action(container, 0);
        return container;
    }

    View tool(String name, String title, int background, Runnable click) {
        LinearLayout row = new LinearLayout(context);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(context.dp(12),0,context.dp(12),0);
        row.setMinimumHeight(context.dp(48));
        row.setContentDescription(title);
        row.addView(icon(name,TEXT),new LinearLayout.LayoutParams(context.dp(14),context.dp(14)));
        TextView label=text(title,12,TEXT,500);
        label.setSingleLine(true);
        label.setPadding(context.dp(8),0,0,0);
        row.addView(label);
        row.setOnClickListener(v -> click.run());
        action(row,background,6);
        return row;
    }

    View section(String name, String title, boolean selected, boolean sidebar, Runnable click) {
        android.widget.FrameLayout frame = new android.widget.FrameLayout(context);
        LinearLayout row=new LinearLayout(context);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(context.dp(12),0,context.dp(12),0);
        row.setMinimumHeight(context.dp(48));
        row.addView(icon(name,TEXT),new LinearLayout.LayoutParams(context.dp(14),context.dp(14)));
        TextView label=text(title,14,TEXT,selected ? 500 : 400);
        label.setSingleLine(true);
        label.setPadding(context.dp(10),0,0,0);
        row.addView(label);
        frame.addView(row,new android.widget.FrameLayout.LayoutParams(-1,-1));
        if (selected) {
            View marker=new View(context);
            marker.setBackground(shape(GREEN,2));
            android.widget.FrameLayout.LayoutParams lp = sidebar
                ? new android.widget.FrameLayout.LayoutParams(context.dp(3),context.dp(22),Gravity.START|Gravity.CENTER_VERTICAL)
                : new android.widget.FrameLayout.LayoutParams(context.dp(18),context.dp(2),Gravity.BOTTOM|Gravity.CENTER_HORIZONTAL);
            frame.addView(marker,lp);
        }
        frame.setContentDescription(title);
        row.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS);
        frame.setSelected(selected);
        frame.setOnClickListener(v -> click.run());
        action(frame,selected ? ACTIVE : SIDEBAR,6);
        return frame;
    }

    static int tint(int color, int amount) {
        int r=((color >>> 16) & 255)*(255-amount)/255 + amount;
        int g=((color >>> 8) & 255)*(255-amount)/255 + amount;
        int b=(color & 255)*(255-amount)/255 + amount;
        return 0xff000000 | (r << 16) | (g << 8) | b;
    }

    View card(String icon, int tileColor, String title, String detail, Runnable select, Runnable menu) {
        return card(icon,tileColor,title,detail,select,menu,false);
    }

    View card(String icon, int tileColor, String title, String detail, Runnable select, Runnable menu, boolean marked) {
        LinearLayout row = new LinearLayout(context);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(context.dp(12), context.dp(8), context.dp(4), context.dp(8));
        row.setMinimumHeight(context.dp(60));
        android.widget.FrameLayout tile = new android.widget.FrameLayout(context);
        tile.setBackground(shape(tileColor, 8));
        tile.addView(icon(icon, 0xffffffff), new android.widget.FrameLayout.LayoutParams(context.dp(20), context.dp(20), Gravity.CENTER));
        row.addView(tile, new LinearLayout.LayoutParams(context.dp(40), context.dp(40)));
        LinearLayout titles = context.column();
        titles.setPadding(context.dp(12), 0, context.dp(8), 0);
        TextView name = text(title, 15, TEXT, 500);
        name.setMaxLines(1);
        name.setEllipsize(android.text.TextUtils.TruncateAt.END);
        titles.addView(name);
        if (detail != null && !detail.isEmpty()) {
            TextView subtitle = text(detail, 12, MUTED, 400);
            subtitle.setMaxLines(1);
            subtitle.setEllipsize(android.text.TextUtils.TruncateAt.END);
            titles.addView(subtitle);
        }
        row.addView(titles, new LinearLayout.LayoutParams(0, -2, 1));
        if (menu != null) {
            row.addView(iconAction("dots","Actions for " + title,menu), new LinearLayout.LayoutParams(context.dp(48), context.dp(48)));
            row.setOnLongClickListener(v -> { menu.run(); return true; });
        }
        if (select != null) row.setOnClickListener(v -> select.run());
        action(row,SURFACE);
        if (marked) {
            GradientDrawable surface=shape(SURFACE,8);
            surface.setStroke(context.dp(1),GREEN);
            row.setBackground(new RippleDrawable(ColorStateList.valueOf(0x28ffffff),surface,shape(0xffffffff,8)));
        }
        return row;
    }
}
