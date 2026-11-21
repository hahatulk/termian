package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.text.Editable;
import android.text.InputType;
import android.text.TextWatcher;
import android.view.View;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.Locale;
import org.json.JSONArray;
import org.json.JSONObject;

final class MobileVault {
    private static final String[] SECTIONS = {"Hosts","Keychain","Port Forwarding","Snippets","Known Hosts","Logs"};
    private static final String[] ICONS = {"hosts","keychain","port_forwarding","snippets","known_hosts","logs"};
    private final MainActivity app;
    private String section = "Hosts", group = "", filter = "";
    private JSONObject data;
    private LinearLayout rows;
    private ScrollView cardScroll;
    private LinearLayout gridRow;
    private boolean grid=true;
    private int columns=1, used=0, sort=0;
    boolean forwarding;

    MobileVault(MainActivity app) { this.app = app; }

    private static JSONArray list(JSONObject data, String name) {
        JSONArray values = data.optJSONArray(name);
        return values == null ? new JSONArray() : values;
    }

    private JSONArray list(String name) { return list(data, name); }

    static String joinTags(JSONArray values) {
        ArrayList<String> tags = new ArrayList<>();
        if (values != null) for (int i=0; i<values.length();i++) tags.add(values.optString(i));
        return String.join(", ", tags);
    }

    private static String value(JSONObject row,String key) {
        return row == null ? "" : MainActivity.fieldValue(row,key);
    }

    String groupId() { return group; }

    private void refresh() { app.refreshVault(); }

    boolean back() {
        if(!section.equals("Hosts") || group.isEmpty()) return false;
        group=value(find("groups",group),"parent");
        refresh();
        return true;
    }

    void launcher(LinearLayout container) {
        cardScroll=null;
        data=app.call("vault");
        ScrollView scroll=new ScrollView(app);
        scroll.setFillViewport(true);
        android.widget.FrameLayout center=new android.widget.FrameLayout(app);
        scroll.addView(center);
        LinearLayout content=app.column();
        content.setPadding(app.dp(16),app.dp(24),app.dp(16),app.dp(16));
        center.addView(content,new android.widget.FrameLayout.LayoutParams(-1,-2,android.view.Gravity.TOP|android.view.Gravity.CENTER_HORIZONTAL));
        center.addOnLayoutChangeListener((v,left,top,right,bottom,oldLeft,oldTop,oldRight,oldBottom) -> {
            int width=Math.min(right-left,app.dp(752));
            if(content.getLayoutParams().width!=width) {
                android.view.ViewGroup.LayoutParams lp=content.getLayoutParams();
                lp.width=width;
                content.setLayoutParams(lp);
            }
        });
        EditText quick=searchField(text("Search hosts or tabs"),true);
        quick.setImeOptions(android.view.inputmethod.EditorInfo.IME_ACTION_GO);
        content.addView(quick,new LinearLayout.LayoutParams(-1,app.dp(48)));
        LinearLayout actions=new LinearLayout(app);
        actions.setPadding(0,app.dp(10),0,app.dp(12));
        actions.addView(app.style.tool("terminal",text("TERMINAL"),TermianStyle.SURFACE,app::localTerminal),
            new LinearLayout.LayoutParams(0,app.dp(48),1));
        actions.addView(app.style.tool("sftp","SFTP",TermianStyle.SURFACE,() -> app.page("sftp")),
            new LinearLayout.LayoutParams(0,app.dp(48),1));
        content.addView(actions);
        LinearLayout results=app.column();
        content.addView(results);
        Runnable update=() -> {
            String query=MainActivity.value(quick).trim();
            results.removeAllViews();
            if(query.isEmpty()) {
                if(list("workspaces").length()>0) {
                    LinearLayout block=launcherBlock(results,text("Workspace templates"));
                    for(int i=0;i<list("workspaces").length();i++) {
                        JSONObject item=list("workspaces").optJSONObject(i);
                        add(block,app.style.card("workspace",TermianStyle.TILE,item.optString("label"),item.optInt("count")+" terminals",
                            () -> app.openWorkspace(item),() -> new AlertDialog.Builder(app).setTitle(text("Delete workspace template?"))
                                .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Delete"),(d,w) ->
                                    app.updateHosts(app.call("deleteWorkspace","id",item.optString("id")))).show()));
                    }
                }
                JSONArray recent=list("recent");
                LinearLayout block=launcherBlock(results,recent.length()==0 ? text("Recent sessions") : text("Recent connections"));
                for(int i=0;i<recent.length();i++) add(block,hostCard(recent.optJSONObject(i),false));
                if(recent.length()==0) block.addView(app.label(text("Recent connections appear here after you open a terminal."),14,TermianStyle.MUTED));
                return;
            }
            LinearLayout block=launcherBlock(results,"Search results");
            for(JSONObject host:matchingHosts(query)) add(block,hostCard(host,false));
            JSONArray tabs=app.terminalTabs();
            for(int i=0;i<tabs.length();i++) {
                JSONObject tab=tabs.optJSONObject(i);
                if(tab!=null && contains(tab.optString("label"),query))
                    add(block,app.style.card("terminal",TermianStyle.TILE,tab.optString("label"),text("Open terminal"),
                        () -> app.selectTab(tab.optString("id")),null));
            }
            add(block,app.style.card("connect",TermianStyle.TILE,text("Quick Connect"),query,() -> app.quickConnect(query),null));
        };
        quick.setOnEditorActionListener((v,id,event) -> {openLauncherQuery(MainActivity.value(quick));return true;});
        watch(quick,update);
        update.run();
        container.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
    }

    private LinearLayout launcherBlock(LinearLayout parent,String title) {
        LinearLayout block=app.column();
        block.setPadding(app.dp(8),app.dp(4),app.dp(8),app.dp(8));
        block.setBackground(app.style.shape(TermianStyle.SURFACE,16));
        block.addView(app.label(title,16,TermianStyle.TEXT));
        add(parent,block);
        return block;
    }

    private EditText searchField(String hint,boolean launcher) {
        EditText search=new EditText(app);
        search.setTextSize(14);
        search.setTypeface(app.style.text("",14,TermianStyle.TEXT,400).getTypeface());
        search.setSingleLine(true);
        search.setContentDescription(hint);
        search.setHint(hint);
        search.setTextColor(TermianStyle.TEXT);
        search.setHintTextColor(TermianStyle.MUTED);
        search.setPadding(app.dp(12),0,app.dp(10),0);
        int background=launcher ? TermianStyle.BAR : TermianStyle.INPUT;
        search.setBackground(app.style.shape(background,launcher ? 10 : 6));
        search.setOnFocusChangeListener((v,focused) -> {
            android.graphics.drawable.GradientDrawable shape=app.style.shape(background,launcher ? 10 : 6);
            shape.setStroke(app.dp(1),focused ? TermianStyle.GREEN : background);
            search.setBackground(shape);
        });
        return search;
    }

    private void watch(EditText search, Runnable changed) {
        search.addTextChangedListener(new TextWatcher() {
            public void beforeTextChanged(CharSequence s,int start,int count,int after) {}
            public void onTextChanged(CharSequence s,int start,int before,int count) { changed.run(); }
            public void afterTextChanged(Editable e) {}
        });
    }

    void render(LinearLayout container) {
        data=app.call("vault");
        grid=app.getPreferences(android.content.Context.MODE_PRIVATE).getBoolean("grid:"+section,true);
        sort=app.getPreferences(android.content.Context.MODE_PRIVATE).getInt("sort:"+section,0);
        LinearLayout page=app.column();
        page.setFocusableInTouchMode(true);
        container.addView(page,new LinearLayout.LayoutParams(-1,-1));

        LinearLayout searchRow=new LinearLayout(app);
        searchRow.setGravity(android.view.Gravity.CENTER_VERTICAL);
        searchRow.setPadding(app.dp(12),app.dp(8),app.dp(12),0);
        boolean hosts=section.equals("Hosts");
        EditText search=searchField(hosts ? text("Find a host or ssh user@hostname...") : "Search "+section,false);
        search.setText(filter);
        searchRow.addView(search,new LinearLayout.LayoutParams(0,app.dp(48),1));
        if(hosts) {
            search.setImeOptions(android.view.inputmethod.EditorInfo.IME_ACTION_GO);
            search.setOnEditorActionListener((text,id,event) -> {openQuery(filter);return true;});
            searchRow.addView(app.style.iconAction("connect",text("Connect"),() -> openQuery(filter)));
        }
        page.addView(searchRow);
        toolbar(page);
        if(hosts && !group.isEmpty()) breadcrumb(page);
        content(page);
        watch(search,() -> {filter=MainActivity.value(search);fill();});
        fill();
    }

    String sectionTitle() { return text(section); }

    void addSections(LinearLayout items, boolean current) {
        for(int i=0;i<SECTIONS.length;i++) {
            String name=SECTIONS[i];
            app.drawerEntry(items,ICONS[i],name,current && section.equals(name),() -> app.showVaultSection(() -> {
                if(!section.equals(name)) {
                    section=name;
                    filter="";
                }
            }),null);
        }
    }

    private void toolbar(LinearLayout page) {
        LinearLayout toolbar=new LinearLayout(app);
        toolbar.setGravity(android.view.Gravity.CENTER_VERTICAL);
        toolbar.setPadding(app.dp(12),app.dp(4),app.dp(8),app.dp(4));
        boolean hosts=section.equals("Hosts");
        LinearLayout left=new LinearLayout(app);
        if(hosts) {
            left.addView(app.style.iconAction("new_host",text("New Host"),() -> app.edit(null)));
            left.addView(app.style.iconAction("caret_down",text("New host or group"),TermianStyle.MUTED,48,11,this::addRecord));
        } else if(!section.equals("Known Hosts") && !section.equals("Logs")) {
            int at=java.util.Arrays.asList(SECTIONS).indexOf(section);
            left.addView(app.style.iconAction(ICONS[at],text("New"),this::addRecord));
        }
        toolbar.addView(left,new LinearLayout.LayoutParams(0,-2,1));
        if(!section.equals("Logs")) toolbar.addView(app.style.iconAction(grid ? "grid" : "list",grid ? text("Grid view") : text("List view"),() -> {
            grid=!grid;
            app.getPreferences(android.content.Context.MODE_PRIVATE).edit().putBoolean("grid:"+section,grid).apply();
            refresh();
        }));
        if(!section.equals("Logs")) toolbar.addView(app.style.iconAction("sorting",text("Sort"),() -> {
            String[] order={text("Newest to oldest"),text("Oldest to newest"),text("A to Z"),text("Z to A")};
            new AlertDialog.Builder(app).setTitle(text("Sort")).setSingleChoiceItems(order,sort,(d,w) -> {
                app.getPreferences(android.content.Context.MODE_PRIVATE).edit().putInt("sort:"+section,w).apply();
                d.dismiss();
                refresh();
            }).show();
        }));
        page.addView(toolbar);
        View divider=new View(app);
        divider.setBackgroundColor(TermianStyle.BAR);
        page.addView(divider,new LinearLayout.LayoutParams(-1,app.dp(1)));
    }

    private void breadcrumb(LinearLayout page) {
        JSONObject g=find("groups",group);
        if(g==null) {group="";return;}
        ArrayList<JSONObject> path=new ArrayList<>();
        for(JSONObject item=g;item!=null && path.size()<32;item=find("groups",value(item,"parent"))) path.add(0,item);
        LinearLayout line=new LinearLayout(app);
        line.setGravity(android.view.Gravity.CENTER_VERTICAL);
        line.addView(app.style.iconAction("arrow_left",text("Parent group"),() -> back()));
        TextView current=app.label(g.optString("label"),14,TermianStyle.TEXT);
        current.setSingleLine(true);
        current.setEllipsize(android.text.TextUtils.TruncateAt.END);
        current.setOnClickListener(view -> {
            String[] labels=new String[path.size()+1];
            labels[0]=text("Personal");
            for(int i=0;i<path.size();i++) labels[i+1]=path.get(i).optString("label");
            new AlertDialog.Builder(app).setTitle(text("Go to group")).setItems(labels,(dialog,i) -> {
                group=i==0 ? "" : path.get(i-1).optString("id");
                refresh();
            }).show();
        });
        app.style.action(current,TermianStyle.BG);
        line.addView(current,new LinearLayout.LayoutParams(0,-2,1));
        line.addView(app.style.iconAction("caret_down",text("Group path"),() -> current.performClick()));
        page.addView(line);
    }

    private void content(LinearLayout container) {
        ScrollView scroll=new ScrollView(app);
        scroll.setFillViewport(true);
        rows=app.column();
        rows.setPadding(app.dp(12),app.dp(12),app.dp(12),app.dp(12));
        scroll.addView(rows);
        cardScroll=scroll;
        columns=1;
        scroll.addOnLayoutChangeListener((v,left,top,right,bottom,oldLeft,oldTop,oldRight,oldBottom) -> {
            if(cardScroll!=scroll) return;
            int count=grid ? Math.max(1,(right-left-app.dp(16))/app.dp(228)) : 1;
            if(count!=columns) {
                columns=count;
                scroll.post(() -> {if(cardScroll==scroll) fill();});
            }
        });
        container.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
    }

    private static boolean contains(String text,String query) {
        return text.toLowerCase(Locale.ROOT).contains(query.toLowerCase(Locale.ROOT).trim());
    }

    private ArrayList<JSONObject> matchingHosts(String query) {
        ArrayList<JSONObject> matches=new ArrayList<>();
        for(JSONObject h:sorted("hosts")) if(contains(h.optString("label"),query) ||
            contains(h.optString("address"),query) || contains(joinTags(h.optJSONArray("tags")),query)) matches.add(h);
        return matches;
    }

    private void openLauncherQuery(String raw) {
        String query=raw.trim();
        JSONArray tabs=app.terminalTabs();
        for(int i=0;i<tabs.length();i++) {
            JSONObject tab=tabs.optJSONObject(i);
            if(tab!=null && tab.optString("label").equalsIgnoreCase(query)) {
                app.selectTab(tab.optString("id"));
                return;
            }
        }
        openQuery(query);
    }

    private void openQuery(String raw) {
        String query=raw.trim();
        if(query.isEmpty()) return;
        ArrayList<JSONObject> hosts=matchingHosts(query);
        if(!hosts.isEmpty()) app.connect(hosts.get(0));
        else app.quickConnect(query);
    }

    private JSONObject find(String kind, String id) {
        JSONArray collection = list(kind);
        for (int i=0;i<collection.length();i++)
            if (value(collection.optJSONObject(i),"id").equals(id)) return collection.optJSONObject(i);
        return null;
    }

    private void add(LinearLayout parent,View view) {
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(-1,-2);
        p.bottomMargin = app.dp(6);
        parent.addView(view,p);
    }

    private boolean matches(String... fields) {
        for(String field:fields) if(contains(field,filter)) return true;
        return filter.trim().isEmpty();
    }

    private ArrayList<JSONObject> sorted(String kind) {
        ArrayList<JSONObject> records=new ArrayList<>();
        JSONArray array=list(kind);
        for(int i=0;i<array.length();i++) {
            JSONObject row=array.optJSONObject(i);
            if(row!=null) records.add(row);
        }
        java.util.Comparator<JSONObject> compare;
        if(sort>=2) {
            compare=(left,right) -> value(left,"label").compareToIgnoreCase(value(right,"label"));
            if(sort==3) compare=compare.reversed();
        } else {
            java.util.Map<String,Integer> recent=new java.util.HashMap<>();
            if(kind.equals("hosts")) {
                JSONArray history=list("recent");
                for(int i=0;i<history.length();i++) recent.put(value(history.optJSONObject(i),"id"),history.length()-i);
            }
            compare=java.util.Comparator.comparingInt((JSONObject row) -> recent.getOrDefault(value(row,"id"),0))
                .thenComparingLong(row -> row.optLong("id",0));
            if(sort==0) compare=compare.reversed();
        }
        if(!kind.equals("known")) records.sort(compare);
        return records;
    }

    private void fill() {
        rows.removeAllViews();
        gridRow=null;
        used=0;
        switch(section) {
            case "Hosts":
                ArrayList<JSONObject> groups=sorted("groups");
                boolean hasGroups=false;
                for(JSONObject g:groups) if(group.equals(value(g,"parent")) && matches(g.optString("label"))) {
                    if(!hasGroups) {header(text("Groups"));hasGroups=true;}
                    card(app.style.card("group",TermianStyle.TILE,g.optString("label"),text("Group"),
                        () -> {group=value(g,"id");filter="";refresh();},() -> groupEditor(g)));
                }
                boolean hasHosts=false;
                for(JSONObject h:matchingHosts(filter)) if(group.equals(value(h,"groupId")) || !filter.isEmpty()) {
                    if(!hasHosts) {header(text("Hosts"));hasHosts=true;}
                    card(hostCard(h,true));
                }
                break;
            case "Keychain":
                header(text("Keys"));collection("keys","keychain",this::keyEditor);
                header(text("Identities"));collection("identities","identity",this::identityEditor);
                break;
            case "Snippets":
                header(text("Packages"));collection("packages","group",this::packageEditor);
                header(text("Snippets"));collection("snippets","snippets",this::snippetMenu);
                break;
            case "Known Hosts":
                collection("known","known_hosts",row -> new AlertDialog.Builder(app).setTitle(row.optString("label"))
                    .setMessage(row.optString("subtitle")+"\\n\\nForget this saved host key?")
                    .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Forget"),(d,w)->app.updateHosts(app.call("deleteKnown","id",value(row,"id")))).show());
                break;
            case "Port Forwarding":
                for(JSONObject r:sorted("rules")) {
                    if(!matches(r.optString("label"),r.optString("subtitle"))) continue;
                    boolean running=r.optBoolean("running");
                    String detail=running ? (r.isNull("activePort") ? "Connecting…" : "Active · "+r.optInt("activePort"))
                        : r.optString("subtitle");
                    card(app.style.card("port_forwarding",running ? 0xff185a3f : TermianStyle.TILE,r.optString("label"),detail,
                        () -> ruleMenu(r),() -> ruleMenu(r)));
                }
                break;
            case "Logs": logRows();break;
        }
        endGrid();
        if(rows.getChildCount()==0) rows.addView(app.label("No "+section.toLowerCase(Locale.ROOT),15,TermianStyle.MUTED));
    }

    private void card(View view) {
        if(!grid || columns==1 || section.equals("Logs")) {add(rows,view);return;}
        if(gridRow==null || used==columns) {
            gridRow=new LinearLayout(app);
            add(rows,gridRow);
            used=0;
        }
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,-2,1);
        if(used>0) lp.leftMargin=app.dp(8);
        gridRow.addView(view,lp);
        used++;
    }

    private void endGrid() {
        while(gridRow!=null && used<columns) {
            LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,1,1);
            lp.leftMargin=app.dp(8);
            gridRow.addView(new View(app),lp);
            used++;
        }
        gridRow=null;
    }

    private void header(String title) {
        endGrid();
        rows.addView(app.label(title,15,TermianStyle.TEXT));
    }

    private View hostCard(JSONObject host,boolean menu) {
        String address = host.optString("address");
        if (address.contains(":")) address = "["+address+"]";
        String tag=joinTags(host.optJSONArray("tags"));
        String username=value(host,"user");
        String detail=(username.isEmpty() ? "" : username+"@")+address;
        if(host.optInt("port",22)!=22) detail+=":"+host.optInt("port");
        if(!tag.isEmpty()) detail+="  ·  "+tag;
        boolean last=list("recent").length()>0 && value(list("recent").optJSONObject(0),"id").equals(value(host,"id"));
        String os=value(host,"os");
        return app.style.card(os.isEmpty()?"new_host":os.equals("debian")||os.equals("ubuntu")?os:"linux",
            os.equals("debian")?0xffd70a53:os.equals("ubuntu")?0xffe95420:TermianStyle.TILE,
            host.optString("label"),detail,() -> app.connect(host),menu ? () -> hostMenu(host) : null,last);
    }

    private void hostMenu(JSONObject host) {
        new AlertDialog.Builder(app).setTitle(host.optString("label")).setItems(new String[]{text("Connect"),text("Connect with SFTP"),text("Edit"),text("Duplicate")},(d,i) -> {
            if (i==0) app.connect(host);
            if (i==1) app.openFiles(host);
            if (i==2) app.edit(host);
            if (i==3) app.updateHosts(app.call("duplicateHost","id",host.optString("id")));
        }).show();
    }

    private void addRecord() {
        switch(section) {
            case "Hosts": new AlertDialog.Builder(app).setTitle(text("New")).setItems(new String[]{text("Host"),text("Group")},(d,i) -> {
                if (i==0) app.edit(null);
                else groupEditor(null);
            }).show(); break;
            case "Keychain": new AlertDialog.Builder(app).setTitle(text("New")).setItems(new String[]{text("Key"),text("Identity")},(d,i) -> {
                if (i==0) keyEditor(null);
                else identityEditor(null);
            }).show(); break;
            case "Snippets": new AlertDialog.Builder(app).setTitle(text("New")).setItems(new String[]{text("Snippet"),text("Package")},(d,i) -> {
                if (i==0) snippetEditor(null);
                else packageEditor(null);
            }).show(); break;
            case "Port Forwarding": ruleEditor(null); break;
        }
    }

    private void collection(String name,String icon,java.util.function.Consumer<JSONObject> action) {
        for(JSONObject row:sorted(name)) {
            String label=row.optString(name.equals("snippets")?"display":"label");
            String detail=name.equals("packages") ? row.optInt("count")+" snippets" : value(row,"subtitle");
            if(matches(label,detail))
                card(app.style.card(icon,TermianStyle.TILE,label,detail,() -> action.accept(row),() -> action.accept(row)));
        }
    }

    private void logRows() {
        JSONArray items=list("logs");
        for(int i=0;i<items.length();i++) {
            JSONObject row=items.optJSONObject(i);
            String detail=row.optString("subtitle")+"  ·  "+row.optString("device");
            if (!row.optString("comment").isEmpty()) detail+="\n"+row.optString("comment");
            if (matches(row.optString("label"),detail))
                add(rows,app.style.card(row.optBoolean("saved")?"bookmark_filled":"logs",TermianStyle.TILE,row.optString("label"),
                    detail, () -> app.openLog(row), () -> logMenu(row)));
        }
        if (items.length()==0) rows.addView(app.label(text("No session logs"),15,TermianStyle.MUTED));
    }

    private void logMenu(JSONObject row) {
        new AlertDialog.Builder(app).setTitle(row.optString("label")).setItems(new String[]{text("Open log"),text("Save comment"),text("Remove bookmark")},(d,i) -> {
            if (i==0) app.openLog(row);
            if (i==1) {
                LinearLayout form=app.column();
                EditText note=field(form,text("Comment"),value(row,"comment"),true);
                editor("Saved comment",form,null,"",() -> app.call("saveLog","id",value(row,"id"),"comment",MainActivity.value(note)));
            }
            if (i==2) app.updateHosts(app.call("unbookmarkLog","id",value(row,"id")));
        }).show();
    }

    private void snippetMenu(JSONObject row) {
        new AlertDialog.Builder(app).setTitle(row.optString("display")).setItems(new String[]{text("Run in selected terminal"),text("Edit")},(d,i) -> {
            if(i==0) new AlertDialog.Builder(app).setTitle(text("Run snippet?")).setMessage(row.optString("script"))
                .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Run"),(a,b) -> app.runSnippet(row)).show();
            else snippetEditor(row);
        }).show();
    }

    private void ruleMenu(JSONObject row) {
        boolean running=row.optBoolean("running");
        new AlertDialog.Builder(app).setTitle(row.optString("label")).setItems(new String[]{running?text("Stop"):text("Start"),text("Edit")},(d,i) -> {
            if(i==0) app.allowNetwork(() -> {
                JSONObject result=app.call("forwardToggle","id",value(row,"id"));
                forwarding=true;
                app.updateHosts(result);
            });
            else ruleEditor(row);
        }).show();
    }

    void forwardEvents(JSONArray events) {
        for(int i=0;i<events.length();i++){
            JSONObject e=events.optJSONObject(i);
            String type=e.optString("type");
            if (type.equals("closed")||type.equals("active")) {
                app.toast(e.optString("value"));
                refresh();
            } else app.handle(e,"forwardAnswer",e.optString("id"));
        }
    }

    private EditText field(LinearLayout form,String label,String value,boolean multi) {
        EditText text=app.field(form,label,value,InputType.TYPE_CLASS_TEXT);
        if(multi) {
            text.setSingleLine(false);
            text.setInputType(InputType.TYPE_CLASS_TEXT|InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
            text.setMinLines(3);
        }
        return text;
    }

    Spinner pick(LinearLayout form,String title,JSONArray choices,String initial) {
        ArrayList<String> names=new ArrayList<>(), ids=new ArrayList<>();
        names.add(text("None")); ids.add("");
        if(choices!=null) for(int i=0;i<choices.length();i++) {
            JSONObject row=choices.optJSONObject(i);
            names.add(row.optString("label"));
            ids.add(row.optString("id"));
        }
        Spinner spin=spinner(form,title,names.toArray(new String[0]));
        spin.setTag(ids);
        spin.setSelection(Math.max(0,ids.indexOf(initial)));
        return spin;
    }

    private Spinner spinner(LinearLayout form,String label,String[] options) {
        form.addView(app.label(label,14,TermianStyle.MUTED));
        Spinner spin=new Spinner(app);
        ArrayAdapter<String> adapter=new ArrayAdapter<>(app,android.R.layout.simple_spinner_dropdown_item,options);
        spin.setAdapter(adapter);
        spin.setContentDescription(label);
        spin.setMinimumHeight(app.dp(48));
        form.addView(spin,new LinearLayout.LayoutParams(-1,-2));
        return spin;
    }

    static String choice(Spinner s) {
        Object values=s.getTag();
        int at=s.getSelectedItemPosition();
        if(values instanceof ArrayList && at>=0) return ((ArrayList<?>)values).get(at).toString();
        return "";
    }

    private void editor(String title,LinearLayout form,String deleteOp,String id,java.util.function.Supplier<JSONObject> save) {
        form.setPadding(app.dp(16),0,app.dp(16),app.dp(12));
        AlertDialog.Builder build=new AlertDialog.Builder(app).setTitle(title).setView(app.scroll(form))
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Save"),null);
        if(deleteOp!=null&&!id.isEmpty()) build.setNeutralButton(text("Delete"),(d,w) ->
            new AlertDialog.Builder(app).setTitle(deleteOp.equals("deleteGroup")?text("Delete group and its hosts?"):text("Delete this record?"))
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Delete"),(a,b)->app.updateHosts(app.call(deleteOp,"id",id))).show());
        AlertDialog dialog=build.show();
        dialog.getButton(-1).setOnClickListener(v -> {
            JSONObject response=save.get();
            if(response==null) return;
            if(response.has("error")) app.toast(response.optString("error"));
            else {dialog.dismiss(); app.updateHosts(response);}
        });
    }

    private void groupEditor(JSONObject row) {
        LinearLayout form=app.column();
        EditText label=field(form,text("Label"),value(row,"label"),false);
        Spinner parent=pick(form,text("Parent group"),list("groups"),row==null?group:value(row,"parent"));
        String id=value(row,"id");
        editor(text("Group"),form,"deleteGroup",id,() -> app.call("saveGroup","id",id,"label",MainActivity.value(label),"parent",choice(parent)));
    }

    private void identityEditor(JSONObject row) {
        LinearLayout form=app.column();
        EditText label=field(form,text("Label"),value(row,"label"),false);
        EditText user=field(form,text("Username"),value(row,"username"),false);
        EditText pass=app.field(form,text("Password"),"",MainActivity.PASSWORD);
        CheckBox clear=app.check(form,"Remove saved password");
        Spinner key=pick(form,text("SSH key"),list("keys"),value(row,"key"));
        String id=value(row,"id");
        editor(text("Identity"),form,"deleteIdentity",id,() -> app.call("saveIdentity","id",id,"label",MainActivity.value(label),
            "username",MainActivity.value(user),"password",MainActivity.value(pass),"clearPassword",clear.isChecked(),"key",choice(key)));
    }

    private void keyEditor(JSONObject row) {
        LinearLayout form=app.column();
        EditText label=field(form,text("Label"),value(row,"label"),false);
        if(row!=null && !row.isNull("publicKey")) form.addView(app.button(text("Copy public key"),() -> {
            ClipboardManager clip=app.getSystemService(ClipboardManager.class);
            clip.setPrimaryClip(ClipData.newPlainText(text("SSH public key"),row.optString("publicKey")));
        }));
        EditText key=field(form,text("Private key"),"",true);
        key.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        EditText pass=app.field(form,text("Passphrase"),"",MainActivity.PASSWORD);
        CheckBox generate=app.check(form,text("Generate ED25519 key"));
        EditText cert=field(form,text("OpenSSH certificate"),value(row,"certificate"),true);
        String id=value(row,"id");
        editor(text("SSH key"),form,"deleteKey",id,() -> app.call("saveKey","id",id,"label",MainActivity.value(label),
            "privateKey",MainActivity.value(key),"password",MainActivity.value(pass),"generate",generate.isChecked(),"certificate",MainActivity.value(cert)));
    }

    private void packageEditor(JSONObject row) {
        LinearLayout form=app.column();
        EditText label=field(form,text("Label"),value(row,"label"),false);
        String id=value(row,"id");
        editor(text("Snippet package"),form,"deletePackage",id,() -> app.call("savePackage","id",id,"label",MainActivity.value(label)));
    }

    private void snippetEditor(JSONObject row) {
        LinearLayout form=app.column();
        EditText label=field(form,text("Label"),value(row,"label"),false);
        EditText script=field(form,"Script",value(row,"script"),true);
        Spinner pack=pick(form,text("Package"),list("packages"),value(row,"package"));
        String id=value(row,"id");
        editor(text("Snippet"),form,"deleteSnippet",id,() -> app.call("saveSnippet","id",id,"label",MainActivity.value(label),
            "script",MainActivity.value(script),"package",choice(pack)));
    }

    private void ruleEditor(JSONObject row) {
        LinearLayout form=app.column();
        EditText label=field(form,text("Label"),value(row,"label"),false);
        String[] kinds={"Local","Remote","Dynamic"};
        Spinner type=spinner(form,text("Type"),java.util.Arrays.stream(kinds).map(MainActivity::text).toArray(String[]::new));
        type.setSelection(value(row,"kind").equals("Remote")?1:value(row,"kind").equals("Dynamic")?2:0);
        Spinner host=pick(form,text("Host"),list("hosts"),value(row,"host"));
        EditText bind=field(form,text("Bind address"),row==null?"127.0.0.1":value(row,"bind"),false);
        EditText port=app.field(form,text("Listening port"),value(row,"port"),InputType.TYPE_CLASS_NUMBER);
        port.setHint(text("0 = choose an available port"));
        LinearLayout target=app.column();
        form.addView(target);
        EditText destination=field(target,text("Destination"),value(row,"destination"),false);
        EditText remote=app.field(target,text("Destination port"),value(row,"destinationPort"),InputType.TYPE_CLASS_NUMBER);
        target.setVisibility(type.getSelectedItemPosition()==2 ? View.GONE : View.VISIBLE);
        type.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener() {
            @Override public void onItemSelected(android.widget.AdapterView<?> parent, View item, int position, long value) {
                target.setVisibility(position==2 ? View.GONE : View.VISIBLE);
            }
            @Override public void onNothingSelected(android.widget.AdapterView<?> parent) {}
        });
        String id=value(row,"id");
        editor(text("Port forwarding rule"),form,"deleteRule",id,() -> {
            if (!validatePort(port,0)) return null;
            boolean dynamic=type.getSelectedItemPosition()==2;
            if (!dynamic) {
                if (MainActivity.value(destination).trim().isEmpty()) {
                    destination.setError(text("Enter a destination address"));
                    destination.requestFocus();
                    return null;
                }
                if (!validatePort(remote,1)) return null;
            }
            return app.call("saveRule","id",id,"kind",kinds[type.getSelectedItemPosition()],
                "label",MainActivity.value(label),"host",choice(host),"bind",MainActivity.value(bind),"port",MainActivity.value(port),
                "destination",dynamic ? "" : MainActivity.value(destination),
                "destinationPort",dynamic ? "" : MainActivity.value(remote));
        });
    }

    private static boolean validatePort(EditText field, int min) {
        try {
            int port=Integer.parseInt(MainActivity.value(field).trim());
            if (port>=min && port<=65535) return true;
        } catch (NumberFormatException ignored) {}
        field.setError("Enter a port from " + min + " to 65535");
        field.requestFocus();
        return false;
    }

    void terminalSettings(JSONObject current) {
        LinearLayout form=app.column();
        EditText font=app.field(form,text("Font size"),value(current,"font"),InputType.TYPE_CLASS_NUMBER);
        String name=value(current,"theme");
        EditText theme=field(form,text("Color theme"),name,false);
        editor(text("Terminal"),form,null,"",() -> {
            JSONObject result=app.call("saveTerminal","font",MainActivity.value(font),"theme",MainActivity.value(theme));
            if (!result.has("error")) app.applyTerminalFont(result.optJSONObject("terminal").optInt("font",14));
            return result;
        });
    }
}
