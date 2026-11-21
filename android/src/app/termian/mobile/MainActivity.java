package app.termian.mobile;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.res.Configuration;
import android.content.pm.PackageManager;
import android.graphics.Typeface;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.view.WindowManager;
import android.view.inputmethod.InputMethodManager;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.PopupMenu;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;
import java.io.File;
import java.util.Arrays;
import org.json.JSONArray;
import org.json.JSONObject;

public final class MainActivity extends Activity {
    static { System.loadLibrary("termian_native"); }

    private static native String nativeCall(String request, byte[] key);
    private static native void setRussian(boolean russian);
    static native String text(String value);
    private static final int BACKGROUND = TermianStyle.BG;
    private static final int ACCENT = TermianStyle.GREEN_TEXT;
    private static final int MUTED = TermianStyle.MUTED;
    private static final String LOCAL_NETWORK = "android.permission.ACCESS_LOCAL_NETWORK";
    private static final int NETWORK_REQUEST = 11;
    static final int PASSWORD = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private FrameLayout appFrame, drawer;
    private LinearLayout root, titleBar, body, hosts, console, drawerRows;
    private ScrollView drawerPanel;
    private TextView pageTitle;
    private View drawerButton, settingsBack;
    private MobileSettings settings;
    private MobileThemes themePicker;
    private String settingsReturn="vault", previewAction="";
    private boolean rightClickPaste, bellSound;
    private LinearLayout hostEditor;
    private java.util.function.BooleanSupplier hostEdited;
    private int terminalBackground = TermianStyle.BAR;
    private int widthDp;
    private MobileVault vault;
    private MobileFiles files;
    private MobileUpdates updates;
    TermianStyle style;
    private JSONArray tabs = new JSONArray();
    private String tabId = "", page = "vault", vaultName = "";
    private boolean wide, locked;
    private LinearLayout privacy;
    private TerminalView terminal, secondaryTerminal;
    private LinearLayout terminalSurface;
    private String secondaryId = "";
    private boolean broadcasting;
    private TextView status;
    private Button ctrlButton;
    private AlertDialog question;
    private Runnable pendingNetwork;
    private boolean opened, active, ready, syncing, control, visible;

    static JSONObject obj(Object... values) {
        JSONObject result = new JSONObject();
        try {
            for (int i = 0; i < values.length; i += 2)
                result.put((String) values[i], values[i + 1] == null ? JSONObject.NULL : values[i + 1]);
            return result;
        } catch (org.json.JSONException error) { throw new IllegalArgumentException(error); }
    }

    JSONObject call(String operation, Object... values) {
        return bridge(operation, null, values);
    }

    private JSONObject bridge(String operation, byte[] key, Object... values) {
        try {
            JSONObject request = obj(values);
            request.put("op", operation);
            return new JSONObject(nativeCall(request.toString(), key));
        } catch (org.json.JSONException | NullPointerException e) {
            return obj("error", text("Не удалось получить ответ Termian."));
        }
    }

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        setRussian(getPreferences(MODE_PRIVATE).getBoolean("ui:russian",false));
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_SECURE);
        style = new TermianStyle(this);
        appFrame = new FrameLayout(this);
        root = column();
        root.setBackgroundColor(BACKGROUND);
        appFrame.addView(root,new FrameLayout.LayoutParams(-1,-1));
        appFrame.setOnApplyWindowInsetsListener((view, insets) -> {
            if (Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets bars = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
                android.graphics.Insets ime = insets.getInsets(WindowInsets.Type.ime());
                view.setPadding(bars.left, bars.top, bars.right, Math.max(bars.bottom, ime.bottom));
            } else {
                view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
                    insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
            }
            return Build.VERSION.SDK_INT >= 30 ? WindowInsets.CONSUMED : insets.consumeSystemWindowInsets();
        });
        setContentView(appFrame);
        if (Build.VERSION.SDK_INT >= 33) getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
            android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT, this::back);
        File data = new File(getNoBackupFilesDir(), "termian");
        try {
            if (!data.isDirectory() && !data.mkdirs()) throw new java.io.IOException(text("Не удалось создать каталог Vault."));
            byte[] key = VaultKey.load(data);
            JSONObject result;
            try { result = bridge("open", key, "root", data.getAbsolutePath()); }
            finally { Arrays.fill(key, (byte) 0); }
            if (result.has("error")) throw new java.io.IOException(result.optString("error"));
            opened = true;
            vaultName=result.optString("vaultName");
            build();
        } catch (Exception e) {
            root.addView(label("Termian", 24, ACCENT));
            root.addView(label(text("Не удалось открыть Vault. Данные не были сброшены."), 16, MUTED));
        }
    }

    @Override public void onStart() {
        super.onStart();
        visible = true;
        if (opened) handler.post(poll);
        if (updates != null) updates.start();
    }

    @Override public void onStop() {
        visible = false;
        handler.removeCallbacks(poll);
        if (updates != null) updates.stop();
        super.onStop();
    }

    @Override public void onDestroy() {
        handler.removeCallbacksAndMessages(null);
        if (question != null) question.dismiss();
        if (updates != null) updates.close();
        if (opened) call("close");
        super.onDestroy();
    }

    @Override public void onConfigurationChanged(Configuration config) {
        super.onConfigurationChanged(config);
        if (opened) {
            readWindowWidth();
            resizeDrawer();
            redrawHosts();
            redrawNavigation();
            arrange();
        }
    }

    private void readWindowWidth() {
        widthDp=getResources().getConfiguration().screenWidthDp;
        wide=widthDp>=720;
    }

    boolean wide() { return wide; }

    int dp(float size) { return Math.round(size * getResources().getDisplayMetrics().density); }

    LinearLayout column() {
        LinearLayout list = new LinearLayout(this);
        list.setOrientation(LinearLayout.VERTICAL);
        return list;
    }

    TextView label(String value, int size, int color) {
        TextView text = style.text(value, size, color, size >= 20 ? 700 : 400);
        text.setPadding(dp(12), dp(8), dp(12), dp(8));
        return text;
    }

    Button button(String text, Runnable action) {
        Button button = new Button(this);
        button.setAllCaps(false);
        button.setText(text);
        button.setMinWidth(dp(48));
        button.setMinHeight(dp(48));
        button.setOnClickListener(v -> action.run());
        button.setTextColor(TermianStyle.TEXT);
        button.setTypeface(style.text("", 14, TermianStyle.TEXT, 500).getTypeface());
        style.action(button, TermianStyle.SURFACE);
        button.setPadding(dp(12), 0, dp(12), 0);
        return button;
    }

    private void build() {
        readWindowWidth();
        titleBar = new LinearLayout(this);
        titleBar.setGravity(Gravity.CENTER_VERTICAL);
        titleBar.setPadding(dp(6),0,dp(4),0);
        titleBar.setBackgroundColor(TermianStyle.BAR);
        root.addView(titleBar,new LinearLayout.LayoutParams(-1,dp(56)));

        drawerButton=style.iconAction("menu",text("Open navigation menu"),this::openDrawer);
        settingsBack=style.iconAction("arrow_left",text("Back"),this::back);
        titleBar.addView(drawerButton);
        titleBar.addView(settingsBack);
        pageTitle=style.text("",16,TermianStyle.TEXT,500);
        pageTitle.setSingleLine(true);
        pageTitle.setEllipsize(android.text.TextUtils.TruncateAt.END);
        titleBar.addView(pageTitle,new LinearLayout.LayoutParams(0,-2,1));

        body=new LinearLayout(this);
        root.addView(body,new LinearLayout.LayoutParams(-1,0,1));
        hosts=column();
        console=column();
        console.setBackgroundColor(TermianStyle.BAR);
        status=style.text("Select a host",12,TermianStyle.MUTED,400);
        status.setPadding(dp(12),0,dp(6),0);
        status.setMaxLines(2);
        LinearLayout terminalBar=new LinearLayout(this);
        terminalBar.setGravity(Gravity.CENTER_VERTICAL);
        terminalBar.addView(status,new LinearLayout.LayoutParams(0,-2,1));
        terminalBar.addView(style.iconAction("workspace",text("Workspace"),this::workspaceMenu));
        console.addView(terminalBar,new LinearLayout.LayoutParams(-1,dp(48)));

        terminal=new TerminalView(this);
        secondaryTerminal=new TerminalView(this);
        applyTerminalSettings(call("terminalSettings"));
        terminalSurface=new LinearLayout(this);
        console.addView(terminalSurface,new LinearLayout.LayoutParams(-1,0,1));
        arrangeTerminals();

        LinearLayout keys=new LinearLayout(this);
        keys.setGravity(Gravity.CENTER_VERTICAL);
        keys.setPadding(dp(4),0,dp(4),0);
        keys.addView(style.iconAction("terminal",text("Keyboard"),this::keyboard));
        ctrlButton=button("Ctrl",() -> setControl(!control));
        ctrlButton.setPadding(dp(9),0,dp(9),0);
        keys.addView(ctrlButton,new LinearLayout.LayoutParams(dp(48),dp(48)));
        for (String[] pair : new String[][] {{"Esc","escape"},{text("Tab"),"tab"}})
            keys.addView(button(pair[0],() -> key(pair[1],null,false,false,false)),new LinearLayout.LayoutParams(dp(48),dp(48)));
        View more=style.iconAction("dots",text("More terminal keys"),() -> {});
        more.setOnClickListener(this::moreTerminalKeys);
        keys.addView(more);
        console.addView(keys);

        vault=new MobileVault(this);
        files=new MobileFiles(this);
        updates=new MobileUpdates(this);
        settings=new MobileSettings(this,updates);
        redrawHosts();
        redrawNavigation();
        arrange();
    }

    private void arrange() {
        body.removeAllViews();
        if (page.equals("settings")) {
            body.addView(settings.view(),new LinearLayout.LayoutParams(-1,-1));
        } else if (page.equals("terminal")) {
            arrangeTerminals();
            body.addView(console,new LinearLayout.LayoutParams(-1,-1));
        } else if (page.equals("sftp")) {
            files.arrange(wide);
            body.addView(files.view,new LinearLayout.LayoutParams(-1,-1));
        } else if (page.equals("vault") && hostEditor!=null) {
            if (widthDp>=900) {
                body.addView(hosts,new LinearLayout.LayoutParams(0,-1,1));
                body.addView(hostEditor,new LinearLayout.LayoutParams(dp(420),-1));
            } else body.addView(hostEditor,new LinearLayout.LayoutParams(-1,-1));
        } else {
            body.addView(hosts,new LinearLayout.LayoutParams(-1,-1));
        }
    }

    private void arrangeTerminals() {
        terminalSurface.removeAllViews();
        boolean split = !secondaryId.isEmpty();
        terminalSurface.setOrientation(wide ? LinearLayout.HORIZONTAL : LinearLayout.VERTICAL);
        terminal.tag(tabId, false);
        secondaryTerminal.tag(secondaryId, true);
        if (split) {
            terminalSurface.addView(terminal,new LinearLayout.LayoutParams(wide ? 0 : -1,wide ? -1 : 0,1));
            View line = new View(this);
            line.setBackgroundColor(TermianStyle.INPUT);
            terminalSurface.addView(line,new LinearLayout.LayoutParams(wide ? dp(2) : -1,wide ? -1 : dp(2)));
            terminalSurface.addView(secondaryTerminal,new LinearLayout.LayoutParams(wide ? 0 : -1,wide ? -1 : 0,1));
        } else terminalSurface.addView(terminal,new LinearLayout.LayoutParams(-1,-1));
    }

    private void newLauncher() { page("new"); }

    void page(String value) {
        closeDrawer();
        if (page.equals("settings") && !value.equals("settings")) updates.detach();
        if (!value.equals("terminal")) ime().hideSoftInputFromWindow(root.getWindowToken(),0);
        page=value;
        redrawHosts();
        redrawNavigation();
        arrange();
    }

    private void redrawNavigation() {
        int bar=page.equals("terminal") ? terminalBackground : TermianStyle.BAR;
        titleBar.setBackgroundColor(bar);
        console.setBackgroundColor(bar);
        boolean inSettings=page.equals("settings");
        drawerButton.setVisibility(inSettings ? View.GONE : View.VISIBLE);
        settingsBack.setVisibility(inSettings ? View.VISIBLE : View.GONE);
        String title=inSettings ? settings.title() : page.equals("sftp") ? "SFTP" : page.equals("new") ? text("New Tab") : vault.sectionTitle();
        if(page.equals("terminal")) {
            title=text("Terminal");
            for(int i=0;i<tabs.length();i++) {
                JSONObject tab=tabs.optJSONObject(i);
                if(tab!=null && tabId.equals(tab.optString("id"))) {
                    title=tab.optString("label",title);
                    break;
                }
            }
        }
        pageTitle.setText(title);
    }

    private void openDrawer() {
        if(drawer!=null || locked) return;
        ime().hideSoftInputFromWindow(root.getWindowToken(),0);
        drawer=new FrameLayout(this);
        View scrim=new View(this);
        scrim.setBackgroundColor(0x99000000);
        scrim.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        scrim.setOnClickListener(v -> closeDrawer());
        drawer.addView(scrim,new FrameLayout.LayoutParams(-1,-1));

        drawerPanel=new ScrollView(this);
        drawerPanel.setBackgroundColor(TermianStyle.SIDEBAR);
        drawerPanel.setFillViewport(true);
        drawerPanel.setFocusableInTouchMode(true);
        drawerPanel.setClickable(true);
        drawerRows=column();
        drawerRows.setPadding(dp(8),dp(8),dp(8),dp(12));
        drawerPanel.addView(drawerRows,new FrameLayout.LayoutParams(-1,-2));
        drawer.addView(drawerPanel,new FrameLayout.LayoutParams(drawerWidth(),-1,Gravity.LEFT));

        root.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS);
        root.setDescendantFocusability(ViewGroup.FOCUS_BLOCK_DESCENDANTS);
        appFrame.addView(drawer,new FrameLayout.LayoutParams(-1,-1));
        drawerMenu();
        drawerPanel.requestFocus();
    }

    private int drawerWidth() { return dp(Math.min(320,Math.max(160,widthDp-48))); }

    private void resizeDrawer() {
        if(drawerPanel!=null) drawerPanel.setLayoutParams(new FrameLayout.LayoutParams(drawerWidth(),-1,Gravity.LEFT));
    }

    private void closeDrawer() {
        if(drawer==null) return;
        appFrame.removeView(drawer);
        drawer=null;
        drawerPanel=null;
        drawerRows=null;
        root.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_AUTO);
        root.setDescendantFocusability(ViewGroup.FOCUS_BEFORE_DESCENDANTS);
    }

    private void drawerMenu() {
        if(drawerRows==null) return;
        int offset=drawerPanel.getScrollY();
        drawerRows.removeAllViews();
        LinearLayout header=new LinearLayout(this);
        header.setGravity(Gravity.CENTER_VERTICAL);
        header.addView(label("Termian",20,ACCENT),new LinearLayout.LayoutParams(0,-2,1));
        header.addView(style.iconAction("close",text("Close navigation menu"),this::closeDrawer));
        drawerRows.addView(header);
        JSONArray entries=settings.appItems();
        for (int i=0;i<entries.length();i++) {
            JSONObject item=entries.optJSONObject(i);
            String title=text(item.optString("title"));
            switch (item.optString("id")) {
                case "settings": drawerEntry(drawerRows,"dots",title,false,this::openSettings,null); break;
                case "vaults": drawerEntry(drawerRows,"vaults",title,page.equals("vault"),() -> page("vault"),null); break;
                case "sftp": drawerEntry(drawerRows,"sftp",title,page.equals("sftp"),() -> page("sftp"),null); break;
                case "quit": drawerEntry(drawerRows,"close",title,false,this::quit,null); break;
            }
        }
        drawerRows.addView(label(text("Vault"),12,MUTED));
        drawerEntry(drawerRows,"vaults",vaultName,false,this::switchVault,null);
        vault.addSections(drawerRows,page.equals("vault"));
        drawerRows.addView(label(text("Sessions"),12,MUTED));
        drawerEntry(drawerRows,"plus",text("New Tab"),page.equals("new"),this::newLauncher,null);
        drawerEntry(drawerRows,"terminal",text("Local Terminal"),false,this::localTerminal,null);
        for(int i=0;i<tabs.length();i++) {
            JSONObject tab=tabs.optJSONObject(i);
            if(tab==null) continue;
            String id=tab.optString("id");
            drawerEntry(drawerRows,"terminal",tab.optString("label"),page.equals("terminal") && id.equals(tabId),
                () -> selectTab(id),() -> closeTab(id));
        }

        drawerRows.addView(label(text("Application"),12,MUTED));
        drawerEntry(drawerRows,"cloud_sync",text("Sync"),false,this::cloudMenu,null);
        drawerEntry(drawerRows,"privacy_lock",text("Lock"),false,this::lock,null);
        if(offset>0) {
            ScrollView panel=drawerPanel;
            panel.post(() -> { if(panel==drawerPanel) panel.scrollTo(0,offset); });
        }
    }

    void drawerEntry(LinearLayout items, String icon, String title, boolean selected, Runnable action, Runnable close) {
        LinearLayout row=new LinearLayout(this);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.addView(style.section(icon,title,selected,() -> { closeDrawer(); action.run(); }),
            new LinearLayout.LayoutParams(0,-1,1));
        if(close!=null) row.addView(style.iconAction("close","Close "+title,TermianStyle.MUTED,48,14,() -> {
            closeDrawer();
            close.run();
        }));
        items.addView(row,new LinearLayout.LayoutParams(-1,dp(48)));
    }

    private void switchVault() {
        if(files!=null && !files.canSwitchVault()) {
            toast(text("Close SFTP and finish file operations before switching Vaults."));
            return;
        }
        confirmDiscard(() -> {
            JSONObject info=call("vaultList");
            JSONArray list=info.optJSONArray("entries");
            if(list==null) { toast(info.optString("error")); return; }
            String[] names=new String[list.length()];
            int current=0;
            for(int i=0;i<list.length();i++) {
                JSONObject entry=list.optJSONObject(i);
                names[i]=entry.optString("name");
                if(entry.optString("id").equals(info.optString("selected"))) current=i;
            }
            new AlertDialog.Builder(this).setTitle(text("Switch Vault"))
                .setSingleChoiceItems(names,current,(dialog,index) -> {
                    String id=list.optJSONObject(index).optString("id");
                    dialog.dismiss();
                    if(!id.equals(info.optString("selected"))) changeVault(call("vaultSelect","id",id));
                }).setNegativeButton(text("Cancel"),null)
                .setPositiveButton(text("Create Vault"),(dialog,index) -> createVault()).show();
        });
    }

    private void createVault() {
        LinearLayout form=column();
        EditText name=field(form,text("Vault name"),"",InputType.TYPE_CLASS_TEXT);
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle(text("Create Vault")).setView(form)
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Create Vault"),null).show();
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(view -> {
            if(files!=null && !files.canSwitchVault()) {
                toast(text("Close SFTP and finish file operations before switching Vaults."));
                return;
            }
            if(changeVault(call("vaultCreate","name",value(name)))) dialog.dismiss();
        });
    }

    private boolean changeVault(JSONObject result) {
        if(result.has("error")) { toast(result.optString("error")); return false; }
        vaultName=result.optString("vaultName");
        pendingNetwork=null;
        hostEditor=null;
        hostEdited=null;
        tabs=new JSONArray();
        tabId="";
        secondaryId="";
        broadcasting=false;
        terminal.clear();
        secondaryTerminal.clear();
        vault=new MobileVault(this);
        applyTerminalSettings(call("terminalSettings"));
        files.refreshChooser();
        page("vault");
        return true;
    }

    void showVaultSection(Runnable select) {
        confirmDiscard(() -> {
            hostEditor=null;
            hostEdited=null;
            select.run();
            page("vault");
        });
    }

    private void moreTerminalKeys(View anchor) {
        PopupMenu menu=new PopupMenu(this,anchor);
        for(String[] item : new String[][] {{"↑","up"},{"↓","down"},{"←","left"},{"→","right"},{"Enter","enter"}})
            menu.getMenu().add(item[0]).setOnMenuItemClickListener(v -> { key(item[1],null,false,false,false); return true; });
        menu.getMenu().add(text("Paste")).setOnMenuItemClickListener(v -> { paste(); return true; });
        menu.show();
    }

    JSONArray terminalTabs() { return tabs; }

    void updateHosts(JSONObject response) {
        if (response.has("error")) toast(response.optString("error"));
        else redrawHosts();
    }

    private void redrawHosts() {
        if(page.equals("new") || page.equals("vault")) {
            hosts.removeAllViews();
            if(page.equals("new")) vault.launcher(hosts);
            else vault.render(hosts);
        }
        if(page.equals("sftp") && files!=null) files.refreshChooser();
    }

    void allowNetwork(Runnable action) {
        if (Build.VERSION.SDK_INT < 37 || checkSelfPermission(LOCAL_NETWORK) == PackageManager.PERMISSION_GRANTED) {
            action.run();
        } else if (pendingNetwork == null) {
            pendingNetwork = action;
            requestPermissions(new String[]{LOCAL_NETWORK}, NETWORK_REQUEST);
        }
    }

    @Override public void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(code, permissions, results);
        if (code != NETWORK_REQUEST) return;
        Runnable action = pendingNetwork;
        pendingNetwork = null;
        if (action != null) action.run();
    }

    void connect(JSONObject host) {
        allowNetwork(() -> useTab(call("connect", "id", host.optString("id"))));
    }

    void quickConnect(String text) {
        allowNetwork(() -> useTab(call("quickConnect","value",text)));
    }

    void openLog(JSONObject row) {
        JSONObject response = call("openLog","id",row.optString("id"));
        if (response.optBoolean("downloading")) { syncing = true; toast(text("Downloading log…")); }
        else useTab(response);
    }

    void openFiles(JSONObject host) { files.open(host); }

    MobileThemes themes() {
        if (themePicker==null) themePicker=new MobileThemes(this);
        return themePicker;
    }

    void applyTerminalSettings(JSONObject state) {
        if (state.has("error")) return;
        int size=state.optInt("font",14);
        terminal.font(size);
        secondaryTerminal.font(size);
        rightClickPaste=state.optBoolean("rightClickPaste");
        bellSound=state.optBoolean("bellSound");
    }

    boolean pasteByMouse(String id) {
        if (!rightClickPaste || id.isEmpty()) return false;
        if (!id.equals(tabId)) focusPane(id);
        paste();
        return true;
    }

    void sftpSettings(LinearLayout card) { files.settings(card); }

    void localTerminal() { useTab(call("local")); }

    void useTab(JSONObject result) {
        if (result.has("error")) { toast(result.optString("error")); return; }
        dismissQuestion();
        terminal.clear();
        tabId = fieldValue(result,"selected");
        secondaryId = fieldValue(result,"secondary");
        broadcasting = result.optBoolean("broadcast");
        secondaryTerminal.clear();
        tabs = result.optJSONArray("tabs");
        active = !tabId.isEmpty();
        setControl(false);
        terminalBackground=TermianStyle.BAR;
        page(active ? "terminal" : "vault");
        updateTerminalTab();
        terminal.reportSize();
        if(!secondaryId.isEmpty()) secondaryTerminal.reportSize();
    }

    private void workspaceMenu() {
        java.util.ArrayList<String> choices=new java.util.ArrayList<>();
        choices.add("Split with a tab");
        if(!secondaryId.isEmpty()) {
            choices.add("Hide second pane");
            choices.add(broadcasting ? "Stop broadcast" : "Broadcast to both panes");
        }
        choices.add("Save workspace");
        new AlertDialog.Builder(this).setTitle(text("Workspace")).setItems(choices.stream().map(MainActivity::text).toArray(String[]::new),(d,w) -> {
            String action=choices.get(w);
            if(action.equals("Hide second pane")) useTab(call("split","id",""));
            else if(action.equals("Broadcast to both panes") || action.equals("Stop broadcast")) {
                useTab(call("broadcast","enabled",!broadcasting));
            } else if(action.equals("Save workspace")) {
                LinearLayout form=column();
                EditText name=field(form,text("Workspace name"),"",InputType.TYPE_CLASS_TEXT);
                new AlertDialog.Builder(this).setTitle(text("Save workspace")).setView(form)
                    .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Save"),(x,y) ->
                        updateHosts(call("saveWorkspace","label",value(name)))).show();
            } else {
                java.util.ArrayList<JSONObject> targets=new java.util.ArrayList<>();
                java.util.ArrayList<String> names=new java.util.ArrayList<>();
                for(int i=0;i<tabs.length();i++) {
                    JSONObject t=tabs.optJSONObject(i);
                    if(!t.optString("id").equals(tabId)) { targets.add(t); names.add(t.optString("label")); }
                }
                if (names.isEmpty()) { toast(text("Open another terminal tab first.")); return; }
                new AlertDialog.Builder(this).setTitle(text("Second pane")).setItems(names.toArray(new String[0]),(x,y) ->
                    useTab(call("split","id",targets.get(y).optString("id")))).show();
            }
        }).show();
    }

    void openWorkspace(JSONObject row) { allowNetwork(() -> useTab(call("openWorkspace","id",row.optString("id")))); }
    void focusPane(String id) { if(!id.isEmpty())selectTab(id); }

    void selectTab(String id) { useTab(call("selectTab", "id", id)); }
    private void closeTab(String id) { useTab(call("disconnect", "id", id)); }

    void updateTerminalTab() {
        ready = false;
        for (int i = 0; i < tabs.length(); i++) {
            JSONObject tab = tabs.optJSONObject(i);
            if (tabId.equals(tab.optString("id"))) {
                ready = tab.optBoolean("ready");
                status.setText(tab.optString("status")+(broadcasting ? "  ·  Broadcast" : ""));
                break;
            }
        }
    }

    @Override public void onBackPressed() { back(); }

    private void back() {
        if (locked) return;
        if(drawer!=null) { closeDrawer(); return; }
        if (opened && page.equals("settings")) {
            if (settings.back()) renderSettings();
            else page(settingsReturn);
            return;
        }
        if (opened && page.equals("vault") && hostEditor!=null) confirmDiscard(this::closeHostEditor);
        else if (opened && !page.equals("vault")) page("vault");
        else if (opened && vault.back()) return;
        else finish();
    }

    private InputMethodManager ime() { return getSystemService(InputMethodManager.class); }

    void keyboard() {
        if (ready) {
            terminal.requestFocus();
            ime().showSoftInput(terminal, InputMethodManager.SHOW_IMPLICIT);
        }
    }

    void typed(String value) {
        if (!ready || value.isEmpty()) return;
        if (control && value.codePointCount(0, value.length()) == 1) key(value, value, false, false, false);
        else call("text", "value", value);
    }

    void key(String key, String typed, boolean ctrl, boolean alt, boolean shift) {
        if (!ready) return;
        call("key", "value", key, "typed", typed, "ctrl", ctrl || control, "alt", alt, "shift", shift);
        if(control) setControl(false);
    }

    private void setControl(boolean enabled) {
        control=enabled;
        ctrlButton.setTextColor(enabled ? ACCENT : TermianStyle.TEXT);
        ctrlButton.setSelected(enabled);
    }

    private void paste() {
        if (!ready) return;
        ClipboardManager clipboard = getSystemService(ClipboardManager.class);
        ClipData clip = clipboard.getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) return;
        CharSequence text = clip.getItemAt(0).getText();
        if (text != null && text.length() <= 262144) call("paste", "value", text.toString());
    }

    private final Runnable poll = new Runnable() {
        @Override public void run() {
            if (!visible || !opened) return;
            if (locked) { handler.postDelayed(this,150); return; }
            if (active || syncing || files.polling || vault.forwarding) {
                JSONObject info = call("poll");
                JSONArray now = info.optJSONArray("tabs");
                if (now != null && !tabs.toString().equals(now.toString())) {
                    tabs = now;
                    if (page.equals("terminal")) updateTerminalTab();
                    redrawNavigation();
                    drawerMenu();
                }
                JSONArray fileEvents = info.optJSONArray("files");
                if (fileEvents != null) files.events(fileEvents);
                JSONArray forwardEvents = info.optJSONArray("forwards");
                if (forwardEvents != null) vault.forwardEvents(forwardEvents);
                JSONObject frame = info.optJSONObject("frame");
                if (frame != null) {
                    terminal.frame(frame);
                    if (bellSound && page.equals("terminal") && frame.optBoolean("bell"))
                        terminal.playSoundEffect(android.view.SoundEffectConstants.CLICK);
                    int bg=0xff000000 | frame.optInt("bg",TermianStyle.BAR);
                    if(bg!=terminalBackground) {
                        terminalBackground=bg;
                        if(page.equals("terminal")) redrawNavigation();
                    }
                }
                JSONObject companion = info.optJSONObject("secondaryFrame");
                if (companion != null) {
                    secondaryTerminal.frame(companion);
                    if (bellSound && page.equals("terminal") && companion.optBoolean("bell"))
                        secondaryTerminal.playSoundEffect(android.view.SoundEffectConstants.CLICK);
                }
                JSONArray events = info.optJSONArray("events");
                if (events != null)
                    for (int i = 0; i < events.length(); i++) handle(events.optJSONObject(i), "answer", tabId);
                JSONObject preview = info.optJSONObject("preview");
                if (preview != null) {
                    syncing = false;
                    showCloudPreview(preview);
                }
                JSONObject log = info.optJSONObject("log");
                if (log != null) {
                    syncing = false;
                    useTab(log);
                }
                JSONObject sync = info.optJSONObject("sync");
                if (sync != null) {
                    syncing = false;
                    updateHosts(sync);
                    if (!sync.has("error")) toast(sync.optString("message"));
                }
            }
            handler.postDelayed(this, active ? 40 : 150);
        }
    };

    private void dismissQuestion() {
        if (question != null) { question.dismiss(); question = null; }
    }

    void handle(JSONObject event, String operation, String id) {
        switch (event.optString("type")) {
            case "status":
                ready = event.optBoolean("ready");
                status.setText(event.optString("value"));
                break;
            case "closed":
                ready = false;
                status.setText(event.optString("value"));
                if (question != null && !question.isShowing()) dismissQuestion();
                break;
            case "changed":
                dismissQuestion();
                question = new AlertDialog.Builder(this).setTitle(text("Ключ сервера изменился"))
                    .setMessage(fingerprint(event) + "\nПодключение отклонено.")
                    .setPositiveButton("OK", null).show();
                break;
            case "trust":
                dismissQuestion();
                question = new AlertDialog.Builder(this).setTitle(text("Проверьте ключ сервера"))
                    .setMessage(fingerprint(event))
                    .setNegativeButton(text("Отклонить"), (d,w) -> sendAnswer(operation, id, "trust", "reject"))
                    .setNeutralButton(text("Один раз"), (d,w) -> sendAnswer(operation, id, "trust", "once"))
                    .setPositiveButton(text("Запомнить"), (d,w) -> sendAnswer(operation, id, "trust", "save"))
                    .setOnCancelListener(d -> sendAnswer(operation, id, "trust", "reject")).show();
                break;
            case "prompt":
                dismissQuestion();
                LinearLayout form = column();
                EditText input = field(form, event.optString("value"), "", event.optBoolean("echo") ? InputType.TYPE_CLASS_TEXT : PASSWORD);
                question = new AlertDialog.Builder(this).setTitle(text("Вход по SSH"))
                    .setView(form)
                    .setNegativeButton(text("Отмена"), (d,w) -> sendAnswer(operation, id, "text", null))
                    .setPositiveButton(text("Продолжить"), (d,w) -> {
                        sendAnswer(operation, id, "text", value(input));
                        input.getText().clear();
                    })
                    .setOnCancelListener(d -> sendAnswer(operation, id, "text", null)).show();
                break;
            default: break;
        }
    }

    private String fingerprint(JSONObject event) {
        return event.optString("host") + " : " + event.optInt("port") + "\n" +
            event.optString("algorithm") + "  SHA256:" + event.optString("fingerprint");
    }

    private void sendAnswer(String operation, String id, String mode, String value) {
        call(operation, "id", id, "mode", mode, "value", value);
    }
    void toast(String message) { Toast.makeText(this, text(message), Toast.LENGTH_LONG).show(); }

    EditText field(LinearLayout form, String label, String initial, int type) {
        form.addView(label(label, 14, MUTED));
        EditText input = new EditText(this);
        input.setTextSize(14);
        input.setTypeface(style.text("",14,TermianStyle.TEXT,400).getTypeface());
        input.setContentDescription(label);
        input.setSingleLine(true);
        input.setInputType(type);
        input.setText(initial);
        input.setSaveEnabled(false);
        input.setTextColor(TermianStyle.TEXT);
        input.setPadding(dp(14), dp(10), dp(14), dp(10));
        input.setMinimumHeight(dp(48));
        input.setBackground(style.shape(TermianStyle.INPUT,6));
        form.addView(input, new LinearLayout.LayoutParams(-1, -2));
        return input;
    }

    CheckBox check(LinearLayout form, String text) {
        CheckBox box = new CheckBox(this);
        box.setText(text);
        form.addView(box);
        return box;
    }

    ScrollView scroll(LinearLayout form) {
        ScrollView scroll = new ScrollView(this);
        scroll.addView(form);
        return scroll;
    }

    static String value(EditText input) { return input.getText().toString(); }

    static String fieldValue(JSONObject record, String field) {
        return record.isNull(field) ? "" : record.optString(field);
    }

    private LinearLayout editorCard(LinearLayout form, String title) {
        LinearLayout card=column();
        card.setPadding(dp(12),dp(4),dp(12),dp(12));
        card.setBackground(style.shape(TermianStyle.SURFACE,8));
        card.addView(label(title,14,TermianStyle.TEXT));
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);
        lp.bottomMargin=dp(12);
        form.addView(card,lp);
        return card;
    }

    private void closeHostEditor() {
        hostEditor=null;
        hostEdited=null;
        ime().hideSoftInputFromWindow(root.getWindowToken(),0);
        arrange();
    }

    private void confirmDiscard(Runnable done) {
        if(hostEdited==null || !hostEdited.getAsBoolean()) {
            done.run();
            return;
        }
        new AlertDialog.Builder(this).setTitle(text("Discard host changes?"))
            .setNegativeButton(text("Keep editing"),null).setPositiveButton(text("Discard"),(d,w) -> done.run()).show();
    }

    void edit(JSONObject host) {
        confirmDiscard(() -> showHostEditor(host));
    }

    private void showHostEditor(JSONObject host) {
        hostEditor=column();
        hostEditor.setBackgroundColor(TermianStyle.BG);
        LinearLayout pane=hostEditor;
        pane.setFocusableInTouchMode(true);
        LinearLayout header=new LinearLayout(this);
        header.setGravity(Gravity.CENTER_VERTICAL);
        header.setBackgroundColor(TermianStyle.SIDEBAR);
        LinearLayout title=column();
        title.setPadding(dp(14),dp(8),0,dp(8));
        title.addView(style.text(host==null ? text("New Host") : text("Host Details"),17,TermianStyle.TEXT,700));
        title.addView(style.text(vaultName,13,MUTED,400));
        header.addView(title,new LinearLayout.LayoutParams(0,-2,1));
        String id=host==null ? "" : host.optString("id");
        if(host!=null) header.addView(style.iconAction("dots",text("Host actions"),() ->
            new AlertDialog.Builder(this).setTitle(host.optString("label")).setItems(new String[]{text("Delete host")},(d,w) ->
                new AlertDialog.Builder(this).setMessage(text("Delete this saved host?"))
                    .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Delete"),(x,y) -> {
                        JSONObject result=call("deleteHost","id",id);
                        if(!result.has("error")) closeHostEditor();
                        updateHosts(result);
                    }).show()).show()));
        header.addView(style.iconAction("collapse_right",text("Close host editor"),() -> confirmDiscard(this::closeHostEditor)));
        pane.addView(header,new LinearLayout.LayoutParams(-1,dp(64)));

        LinearLayout form=column();
        form.setPadding(dp(14),dp(14),dp(14),0);
        LinearLayout addressCard=editorCard(form,text("Address"));
        EditText address=field(addressCard,text("IP or Hostname"),host==null ? "" : host.optString("address"),
            InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        LinearLayout general=editorCard(form,text("General"));
        EditText name=field(general,text("Label"),host==null ? "" : host.optString("label"),InputType.TYPE_CLASS_TEXT);
        JSONObject data=call("vault");
        android.widget.Spinner group=vault.pick(general,text("Parent Group"),data.optJSONArray("groups"),host==null ? vault.groupId() : fieldValue(host,"groupId"));
        EditText tags=field(general,text("Tags (comma-separated)"),host==null ? "" : MobileVault.joinTags(host.optJSONArray("tags")),InputType.TYPE_CLASS_TEXT);
        LinearLayout ssh=editorCard(form,"SSH");
        EditText port=field(ssh,text("Port"),host==null ? "22" : String.valueOf(host.optInt("port")),InputType.TYPE_CLASS_NUMBER);
        ssh.addView(label(text("Credentials"),14,TermianStyle.TEXT));
        android.widget.Spinner identity=vault.pick(ssh,text("Identity"),data.optJSONArray("identities"),host==null ? "" : fieldValue(host,"identityId"));
        LinearLayout personal=column();
        ssh.addView(personal);
        EditText user=field(personal,text("Username"),host==null ? "" : fieldValue(host,"user"),InputType.TYPE_CLASS_TEXT);
        EditText password=field(personal,text("SSH password"),"",PASSWORD);
        CheckBox clear=host==null ? null : check(personal,"Remove saved password");
        android.widget.Spinner key=vault.pick(personal,text("SSH key"),data.optJSONArray("keys"),host==null ? "" : fieldValue(host,"keyId"));
        personal.setVisibility(MobileVault.choice(identity).isEmpty() ? View.VISIBLE : View.GONE);
        identity.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener() {
            @Override public void onItemSelected(android.widget.AdapterView<?> parent, View view, int position, long keyId) {
                personal.setVisibility(MobileVault.choice(identity).isEmpty() ? View.VISIBLE : View.GONE);
            }
            @Override public void onNothingSelected(android.widget.AdapterView<?> parent) {}
        });
        String originalTheme=host==null ? "" : fieldValue(host,"theme");
        LinearLayout appearance=editorCard(form,text("Terminal theme"));
        MobileThemes.Choice theme=themes().new Choice(originalTheme,call("terminalSettings").optString("theme"),nameValue -> true);
        appearance.addView(theme);
        String originalAddress=value(address), originalName=value(name), originalPort=value(port), originalTags=value(tags), originalUser=value(user);
        String originalGroup=MobileVault.choice(group), originalIdentity=MobileVault.choice(identity), originalKey=MobileVault.choice(key);
        hostEdited=() -> !value(address).equals(originalAddress) || !value(name).equals(originalName) ||
            !value(port).equals(originalPort) || !value(tags).equals(originalTags) || !value(user).equals(originalUser) ||
            !MobileVault.choice(group).equals(originalGroup) || !MobileVault.choice(identity).equals(originalIdentity) ||
            !MobileVault.choice(key).equals(originalKey) || !theme.value().equals(originalTheme) ||
            password.length()!=0 || clear!=null && clear.isChecked();
        pane.addView(scroll(form),new LinearLayout.LayoutParams(-1,0,1));

        LinearLayout footer=new LinearLayout(this);
        footer.setGravity(Gravity.END|Gravity.CENTER_VERTICAL);
        footer.setPadding(dp(10),dp(4),dp(10),dp(4));
        footer.setBackgroundColor(TermianStyle.SIDEBAR);
        footer.addView(button(text("Cancel"),() -> confirmDiscard(this::closeHostEditor)));
        footer.addView(style.tool("check",text("Save"),TermianStyle.GREEN,() -> {
            if(value(address).trim().isEmpty()) {address.setError(text("Enter a host address"));address.requestFocus();return;}
            JSONObject result=call("saveHost","id",id,"label",value(name),"address",value(address),
                "port",value(port),"user",value(user),"password",value(password),
                "clearPassword",clear!=null && clear.isChecked(),"group",MobileVault.choice(group),
                "identity",MobileVault.choice(identity),"key",MobileVault.choice(key),"tags",value(tags),
                "theme",theme.value().equals(originalTheme) ? null : theme.value());
            if(!result.has("error")) { password.getText().clear(); closeHostEditor(); }
            updateHosts(result);
        }));
        pane.addView(footer,new LinearLayout.LayoutParams(-1,dp(56)));
        if(!page.equals("vault")) page("vault"); else arrange();
        pane.requestFocus();
    }

    void cloudMenu() {
        if (syncing) { toast(text("Cloud operation in progress.")); return; }
        JSONObject config = call("cloud");
        if (config.isNull("url")) { cloudSettings(config); return; }
        new AlertDialog.Builder(this).setTitle(text("Self-hosted cloud")).setItems(new String[]{text("Check changes"), text("Settings")}, (d,w) -> {
            if (w == 1) cloudSettings(config);
            else allowNetwork(() -> cloudOperation("cloudPreview"));
        }).show();
    }

    void editCloudSettings() {
        if (syncing) toast(text("Cloud operation in progress."));
        else cloudSettings(call("cloud"));
    }

    void cloudAction(String operation) {
        if (syncing) { toast(text("Cloud operation in progress.")); return; }
        allowNetwork(() -> {
            previewAction=operation.startsWith("force") ? operation : "";
            cloudOperation(previewAction.isEmpty() ? operation : "cloudPreview");
        });
    }

    private void cloudOperation(String operation) {
        if (syncing) { toast(text("Cloud operation in progress.")); return; }
        JSONObject result = call(operation);
        if (result.has("error")) {
            if (operation.equals("cloudPreview")) previewAction="";
            toast(result.optString("error"));
        }
        else { syncing = true; toast(operation.equals("cloudPreview") ? text("Checking changes…") : text("Sync in progress…")); }
    }

    private void showCloudPreview(JSONObject result) {
        String requested=previewAction;
        previewAction="";
        if (result.has("error")) {
            new AlertDialog.Builder(this).setTitle(text("Sync")).setMessage(result.optString("error"))
                .setNegativeButton(text("Close"),null).setPositiveButton(text("Check again"),(d,w) -> cloudOperation("cloudPreview")).show();
            return;
        }
        StringBuilder summary = new StringBuilder(result.optString("message"));
        JSONArray changes = result.optJSONArray("changes");
        if (changes != null) for (int i = 0; i < changes.length(); i++) summary.append("\n").append(changes.optString(i));
        if (!requested.isEmpty() && result.optBoolean(requested.equals("forceUpload") ? "canUpload" : "canDownload")) {
            confirmCloudReplacement(requested,summary.toString());
            return;
        }
        java.util.ArrayList<String> labels = new java.util.ArrayList<>();
        labels.add(text("Sync")); labels.add(text("Check again"));
        if (result.optBoolean("canUpload")) labels.add(text("Force upload"));
        if (result.optBoolean("canDownload")) labels.add(text("Force download"));
        new AlertDialog.Builder(this).setTitle(text("Sync preview")).setMessage(summary.toString())
            .setNegativeButton(text("Close"),null).setPositiveButton(text("Sync"),(d,w) -> cloudOperation("sync"))
            .setNeutralButton(text("More"),(d,w) -> new AlertDialog.Builder(this).setTitle(text("Cloud actions"))
                .setItems(labels.toArray(new String[0]),(x,y) -> {
                    String action = labels.get(y);
                    if (y < 2) cloudOperation(y == 0 ? "sync" : "cloudPreview");
                    else confirmCloudReplacement(action.equals(text("Force upload")) ? "forceUpload" : "forceDownload", "");
                }).show()).show();
    }

    private void confirmCloudReplacement(String operation, String summary) {
        boolean upload=operation.equals("forceUpload");
        String warning=text(upload ? "Replace the cloud snapshot with this device's Vault?"
            : "Back up this device's Vault and replace its synced records with the cloud snapshot?");
        new AlertDialog.Builder(this).setTitle(text(upload ? "Force upload" : "Force download"))
            .setMessage(summary.isEmpty() ? warning : summary + "\n\n" + warning)
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Confirm"),(d,w) -> cloudOperation(operation)).show();
    }

    private void cloudSettings(JSONObject config) {
        LinearLayout form = column();
        EditText url = field(form, text("HTTPS-адрес сервера"), fieldValue(config, "url"), InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        EditText token = field(form, text("Токен"), "", PASSWORD);
        EditText secret = field(form, text("Пароль снимка"), "", PASSWORD);
        CheckBox empty = check(form, text("Использовать пустой пароль снимка"));
        EditText user = field(form, text("Логин Basic auth"), fieldValue(config, "basicUser"), InputType.TYPE_CLASS_TEXT);
        EditText password = field(form, text("Пароль Basic auth"), "", PASSWORD);
        if (!config.isNull("url")) form.addView(label(text("Пустые поля секретов сохраняют прежние значения для этого сервера."), 14, MUTED));
        AlertDialog dialog = new AlertDialog.Builder(this).setTitle(text("Своё облако")).setView(scroll(form))
            .setNegativeButton(text("Отмена"), null).setPositiveButton(text("Сохранить"), null).show();
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(v -> {
            JSONObject result = call("configureCloud", "url", value(url), "token", value(token), "passphrase", value(secret),
                "emptyPassphrase", empty.isChecked(), "basicUser", value(user), "basicPassword", value(password));
            if (result.has("error")) toast(result.optString("error"));
            else {
                token.getText().clear();
                secret.getText().clear();
                password.getText().clear();
                dialog.dismiss();
                renderSettings();
            }
        });
    }
    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request,result,data);
        if (files != null) files.result(request,result,data);
    }

    void runSnippet(JSONObject snippet) {
        JSONObject result = call("snippetRun", "id", snippet.optString("id"));
        if (result.has("error")) toast(result.optString("error"));
        else page("terminal");
    }

    void refreshVault() { redrawHosts(); }

    private void openSettings() {
        if (page.equals("settings")) return;
        settingsReturn=page;
        settings.open();
        page("settings");
    }

    void renderSettings() {
        if (!page.equals("settings")) return;
        redrawNavigation();
        arrange();
    }

    void setLanguage(boolean russian) {
        if (getPreferences(MODE_PRIVATE).getBoolean("ui:russian",false)==russian) return;
        getPreferences(MODE_PRIVATE).edit().putBoolean("ui:russian",russian).apply();
        setRussian(russian);
        vaultName=call("vaultList").optString("vaultName",vaultName);
        drawerButton.setContentDescription(text("Open navigation menu"));
        settingsBack.setContentDescription(text("Back"));
        renderSettings();
    }

    private void quit() {
        if (files.documentOperation()) { toast(text("Finish the current file operation first.")); return; }
        if (syncing) { toast(text("Cloud operation in progress.")); return; }
        confirmDiscard(() -> new AlertDialog.Builder(this).setTitle(text("Quit Termian"))
            .setMessage(text("Close all sessions and quit Termian?"))
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Quit Termian"),(d,w) -> finishAndRemoveTask()).show());
    }

    @Override public boolean dispatchKeyEvent(android.view.KeyEvent event) {
        if (opened && settings!=null && !locked && drawer==null && !page.equals("settings")
            && !(getCurrentFocus() instanceof EditText) && event.getAction()==android.view.KeyEvent.ACTION_DOWN) {
            String action=settings.shortcut(event);
            if (!action.isEmpty() && runShortcut(action,event.getRepeatCount()==0)) return true;
        }
        return super.dispatchKeyEvent(event);
    }

    private boolean runShortcut(String action, boolean run) {
        boolean current=page.equals("terminal") && !tabId.isEmpty();
        switch (action) {
            case "pasteToTerminal": if (!current) return false; if (run) paste(); return true;
            case "localTerminal": if (run) localTerminal(); return true;
            case "close": if (!current) return false; if (run) closeTab(tabId); return true;
            case "bottom": case "top":
                if (tabs.length()==0) return false;
                if (run) cycleTerminal(action.equals("bottom") ? 1 : -1);
                return true;
            case "forwarding": if (run) vault.openSection("Port Forwarding"); return true;
            case "newTab": if (run) newLauncher(); return true;
            case "toggleSplitViewBroadcastMode":
                if (!current || secondaryId.isEmpty()) return false;
                if (run) useTab(call("broadcast","enabled",!broadcasting));
                return true;
            case "snippets": if (run) vault.openSection("Snippets"); return true;
            default: return false;
        }
    }

    private void cycleTerminal(int step) {
        int current=-1;
        for (int i=0;i<tabs.length();i++) if (tabId.equals(tabs.optJSONObject(i).optString("id"))) current=i;
        int index=current<0 ? (step>0 ? 0 : tabs.length()-1) : (current+step+tabs.length())%tabs.length();
        selectTab(tabs.optJSONObject(index).optString("id"));
    }

    void privacyPassword(boolean thenLock) {
        LinearLayout form = column();
        boolean set = call("settingsState").optBoolean("privacyConfigured");
        EditText current = set ? field(form,text("Current password"),"",PASSWORD) : null;
        EditText password = field(form,text("Lock password"),"",PASSWORD);
        AlertDialog dialog = new AlertDialog.Builder(this).setTitle(text("Privacy lock")).setView(form)
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Save"),null).show();
        dialog.getButton(-1).setOnClickListener(v -> {
            JSONObject r = call("savePrivacy","password",value(password),"current",current == null ? "" : value(current));
            if (r.has("error")) toast(r.optString("error"));
            else { dialog.dismiss(); if (thenLock) lock(); }
        });
    }

    boolean privacyLocked() { return locked; }

    private void lock() {
        if (updates != null) updates.detach();
        if (!call("settingsState").optBoolean("privacyConfigured")) { privacyPassword(true); return; }
        dismissQuestion();
        locked = true;
        privacy = column();
        privacy.setGravity(Gravity.CENTER);
        privacy.setBackgroundColor(BACKGROUND);
        privacy.setPadding(dp(30), dp(24), dp(30), dp(24));
        privacy.addView(style.icon("privacy_lock",ACCENT), new LinearLayout.LayoutParams(dp(42),dp(42)));
        privacy.addView(label(text("Termian is locked"),24,TermianStyle.TEXT));
        EditText password = field(privacy,text("Password"),"",PASSWORD);
        privacy.addView(button(text("Unlock"),() -> {
            JSONObject r = call("unlock","password",value(password));
            if (r.has("error")) { toast(r.optString("error")); return; }
            locked = false;
            setContentView(appFrame);
            if (!tabId.isEmpty()) useTab(call("selectTab","id",tabId));
            redrawHosts();
            renderSettings();
        }));
        setContentView(privacy);
    }

}
