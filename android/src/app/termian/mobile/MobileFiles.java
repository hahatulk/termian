package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.text.InputType;
import android.view.View;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import org.json.JSONArray;
import org.json.JSONObject;

final class MobileFiles {
    private static final int PICK = 41;
    private static final int CREATE = 42;
    private static final int PICK_TREE = 43;
    private static final int SAVE_TREE = 44;
    private final MainActivity app;
    final LinearLayout view;
    private final LinearLayout remote;
    private LinearLayout rows;
    private TextView activity, path;
    private View backFolder, forwardFolder;
    private String export, exportName;
    private boolean ready, documentBusy, showHidden;
    private boolean chooseAppAfterSave;
    private MobileFileReader reader;
    boolean polling;

    MobileFiles(MainActivity app) {
        this.app = app;
        showHidden=app.getPreferences(Activity.MODE_PRIVATE).getBoolean("sftp:showHidden",false);
        view=app.column();
        remote=app.column();
        refreshChooser();
    }

    boolean canSwitchVault() { return !documentBusy && !polling; }

    boolean documentOperation() { return documentBusy; }

    void settings(LinearLayout card) {
        card.addView(app.label(text("Android opens saved files with apps chosen in the system file menu."),14,TermianStyle.MUTED));
        Switch hidden=new Switch(app);
        hidden.setText(text("Show Hidden Files"));
        hidden.setMinHeight(app.dp(48));
        hidden.setChecked(showHidden);
        card.addView(hidden,new LinearLayout.LayoutParams(-1,-2));
        hidden.setOnCheckedChangeListener((button, enabled) -> {
            if (!setHidden(enabled)) app.renderSettings();
        });
    }

    private boolean setHidden(boolean enabled) {
        if (polling) {
            JSONObject response=app.call("filesHidden","enabled",enabled);
            if (error(response)) return false;
            if (response.has("entries")) listing(response);
        }
        showHidden=enabled;
        app.getPreferences(Activity.MODE_PRIVATE).edit().putBoolean("sftp:showHidden",enabled).apply();
        return true;
    }

    void arrange(boolean wide) {
        view.removeAllViews();
        if (reader != null) {
            view.setOrientation(LinearLayout.VERTICAL);
            view.addView(reader.view, new LinearLayout.LayoutParams(-1, -1));
            return;
        }
        view.setOrientation(wide ? LinearLayout.HORIZONTAL : LinearLayout.VERTICAL);
        if(wide) {
            LinearLayout local=app.column();
            local.setBackgroundColor(TermianStyle.SIDEBAR);
            local.setPadding(app.dp(12),app.dp(8),app.dp(12),app.dp(8));
            local.addView(app.label(text("Local"),18,TermianStyle.TEXT));
            local.addView(app.label(text("Choose Android files or folders to upload."),13,TermianStyle.MUTED));
            local.addView(app.style.tool("upload",text("SELECT FILE"),TermianStyle.SURFACE,() -> chooseLocal(false)));
            local.addView(app.style.tool("group",text("SELECT FOLDER"),TermianStyle.SURFACE,() -> chooseLocal(true)));
            view.addView(local,new LinearLayout.LayoutParams(0,-1,1));
            View divider=new View(app);
            divider.setBackgroundColor(TermianStyle.BAR);
            view.addView(divider,new LinearLayout.LayoutParams(app.dp(1),-1));
        }
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(wide ? 0 : -1,wide ? -1 : 0,1);
        view.addView(remote,lp);
    }

    private void chooseLocal(boolean tree) {
        if(!ready) { app.toast(text("Connect to an SSH host first.")); return; }
        pick(tree);
    }

    void refreshChooser() {
        if(polling || documentBusy) return;
        remote.removeAllViews();
        LinearLayout header=new LinearLayout(app);
        header.setGravity(android.view.Gravity.CENTER_VERTICAL);
        header.setBackgroundColor(TermianStyle.SIDEBAR);
        header.addView(app.label("SFTP",16,TermianStyle.TEXT),new LinearLayout.LayoutParams(0,-2,1));
        remote.addView(header,new LinearLayout.LayoutParams(-1,app.dp(48)));
        remote.addView(app.label(text("Connect to host"),18,TermianStyle.TEXT));
        remote.addView(app.label(text("Select a saved host to browse its files."),13,TermianStyle.MUTED));
        ScrollView scroll=new ScrollView(app);
        LinearLayout items=app.column();
        items.setPadding(app.dp(12),0,app.dp(12),0);
        scroll.addView(items);
        JSONArray hosts=app.call("vault").optJSONArray("hosts");
        if(hosts!=null) for(int i=0;i<hosts.length();i++) {
            JSONObject host=hosts.optJSONObject(i);
            add(items,app.style.card("new_host",TermianStyle.TILE,host.optString("label"),host.optString("address"),() -> open(host),null));
        }
        remote.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
    }

    void open(JSONObject host) {
        if (documentBusy) return;
        if (polling) {
            new AlertDialog.Builder(app).setTitle(text("Switch SFTP host?")).setMessage(text("The current SFTP connection and its transfers will close."))
                .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Switch"),(d,w) -> start(host)).show();
        } else start(host);
    }

    private void start(JSONObject host) {
        app.allowNetwork(() -> {
            JSONObject response=app.call("filesOpen","id",host.optString("id"),"showHidden",showHidden);
            if(error(response))return;
            closePreview();
            polling=true;
            ready=false;
            chooseAppAfterSave=false;
            remote.removeAllViews();
            LinearLayout header=new LinearLayout(app);
            header.setGravity(android.view.Gravity.CENTER_VERTICAL);
            header.setBackgroundColor(TermianStyle.SIDEBAR);
            TextView name=app.label(host.optString("label"),16,TermianStyle.TEXT);
            name.setMaxLines(1);
            header.addView(name,new LinearLayout.LayoutParams(0,-2,1));
            header.addView(app.style.iconAction("upload",text("Upload"),() -> new AlertDialog.Builder(app).setTitle(text("Upload"))
                .setItems(new String[]{text("File"),text("Folder")},(d,w)->pick(w==1)).show()));
            header.addView(app.style.iconAction("close",text("Disconnect"),this::close));
            remote.addView(header,new LinearLayout.LayoutParams(-1,app.dp(48)));
            activity=app.label("Connecting…",14,TermianStyle.MUTED);
            remote.addView(activity);
            path=app.label("/",14,TermianStyle.TEXT);
            path.setMaxLines(2);
            remote.addView(path);
            LinearLayout actions=new LinearLayout(app);
            backFolder=app.style.iconAction("sftp_arrow",text("Back folder"),() -> run("filesBack"));
            forwardFolder=app.style.iconAction("sftp_arrow",text("Forward folder"),() -> run("filesForward"));
            forwardFolder.setRotation(180);
            navEnabled(backFolder,false);
            navEnabled(forwardFolder,false);
            actions.addView(backFolder);
            actions.addView(forwardFolder);
            actions.addView(app.button("Up",() -> run("filesUp")),new LinearLayout.LayoutParams(0,app.dp(48),1));
            actions.addView(app.button(text("Refresh"),() -> run("filesRefresh")),new LinearLayout.LayoutParams(0,app.dp(48),1));
            View more=app.style.iconAction("dots",text("File actions"),() -> {});
            more.setOnClickListener(anchor -> {
                android.widget.PopupMenu menu=new android.widget.PopupMenu(app,anchor);
                menu.getMenu().add(text("New Folder")).setOnMenuItemClickListener(v -> { folderDialog(null); return true; });
                menu.getMenu().add(text(showHidden ? "Hide Hidden Files" : "Show Hidden Files"))
                    .setOnMenuItemClickListener(v -> { setHidden(!showHidden); return true; });
                menu.getMenu().add(text("Cancel transfer")).setOnMenuItemClickListener(v -> { run("filesControl","retry",false); return true; });
                menu.show();
            });
            actions.addView(more);
            remote.addView(actions);
            ScrollView scroll=new ScrollView(app);
            rows=app.column();
            rows.setPadding(app.dp(12),app.dp(8),app.dp(12),app.dp(8));
            scroll.addView(rows);
            remote.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
            app.page("sftp");
        });
    }

    private void close() {
        if (documentBusy) {app.toast(text("Finish the file picker or save operation first."));return;}
        closePreview();
        app.call("filesClose");
        polling=false;
        ready=false;
        refreshChooser();
    }

    private void add(LinearLayout into,android.view.View view) {
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);
        lp.bottomMargin=app.dp(5);
        into.addView(view,lp);
    }

    private boolean error(JSONObject result) {
        if (!result.has("error")) return false;
        app.toast(result.optString("error"));
        return true;
    }

    private void run(String operation,Object... values) {
        if (!ready) { app.toast(text("Connect to an SFTP host first.")); return; }
        if (documentBusy) { app.toast(text("Finish the current file operation first.")); return; }
        error(app.call(operation,values));
    }

    private void folderDialog(JSONObject file) {
        LinearLayout form=app.column();
        EditText text=app.field(form,text("Name"),file==null?"":file.optString("name"),InputType.TYPE_CLASS_TEXT);
        new AlertDialog.Builder(app).setTitle(file==null?text("New folder"):text("Rename")).setView(form)
            .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Save"),(d,w) -> {
                String name=MainActivity.value(text);
                if(file==null)run("filesMkdir","newName",name);
                else run("filesRename","name",file.optString("name"),"newName",name);
            }).show();
    }

    private void navEnabled(View button,boolean enabled) {
        button.setEnabled(enabled);
        button.setAlpha(enabled ? 1f : .5f);
    }

    private void download(String name, boolean chooser) {
        if (!ready || documentBusy) { app.toast(text("Finish the current file operation first.")); return; }
        if (!error(app.call("filesDownload", "name", name))) chooseAppAfterSave = chooser;
    }

    private void preview(String name) {
        if (!ready || documentBusy || reader != null) return;
        JSONObject response = app.call("filesPreview", "name", name, "offset", 0);
        if (error(response)) return;
        reader = new MobileFileReader(app, name, response.optString("request"), this::closePreview);
        arrange(app.wide());
    }

    boolean closePreview() {
        if (reader == null) return false;
        reader = null;
        arrange(app.wide());
        return true;
    }

    private void fileMenu(JSONObject file) {
        boolean folder=file.optBoolean("folder");
        boolean link=file.optBoolean("link");
        String name=file.optString("name");
        java.util.ArrayList<String> options=new java.util.ArrayList<>();
        if (!link) {
            options.add("Open");
            if (!folder) options.add("Open with...");
            options.add("Copy to target directory");
        }
        options.add("Rename");
        options.add("Delete");
        if (!link) options.add("Edit Permissions");
        String[] labels=options.stream().map(MainActivity::text).toArray(String[]::new);
        new AlertDialog.Builder(app).setTitle(name).setItems(labels,(d,w) -> {
            switch (options.get(w)) {
                case "Open": if (folder) run("filesFolder","name",name); else preview(name); break;
                case "Open with...": download(name, true); break;
                case "Copy to target directory": download(name, false); break;
                case "Rename": folderDialog(file); break;
                case "Edit Permissions":
                    LinearLayout form=app.column();
                    EditText mode=app.field(form,text("Octal mode"),MainActivity.fieldValue(file,"mode"),InputType.TYPE_CLASS_NUMBER);
                    new AlertDialog.Builder(app).setTitle(text("Edit Permissions")).setView(form)
                        .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Apply"),(a,b) ->
                            run("filesChmod","name",name,"mode",MainActivity.value(mode))).show();
                    break;
                case "Delete":
                    new AlertDialog.Builder(app).setTitle(text("Delete") + ": " + name)
                        .setMessage(folder&&!link?text("This deletes the folder and its contents."): text("This deletes the remote entry."))
                        .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Delete"),(a,b) -> run("filesDelete","name",name)).show();
                    break;
            }
        }).show();
    }

    private void listing(JSONObject listing) {
        path.setText(listing.optString("path"));
        navEnabled(backFolder,listing.optBoolean("canBack"));
        navEnabled(forwardFolder,listing.optBoolean("canForward"));
        rows.removeAllViews();
        JSONArray entries=listing.optJSONArray("entries");
        for (int j=0;j<entries.length();j++) {
            JSONObject f=entries.optJSONObject(j);
            boolean link=f.optBoolean("link");
            boolean dir=f.optBoolean("folder") && !link;
            String name=f.optString("name");
            add(rows,app.style.fileCard(dir ? "folder" : link ? "link" : "file", name, dir ? text("Folder") : f.optString("size"),
                () -> {
                    if (dir) run("filesFolder", "name", name);
                    else if (link) fileMenu(f);
                    else preview(name);
                }, () -> fileMenu(f), f.optBoolean("hidden")));
        }
        if (entries.length()==0) rows.addView(app.label(text("Folder is empty"),15,TermianStyle.MUTED));
    }

    void events(JSONArray events) {
        for(int i=0;i<events.length();i++) {
            JSONObject e=events.optJSONObject(i);
            switch(e.optString("type")) {
                case "connected": ready=true;activity.setText(text("Connected"));break;
                case "listing": listing(e); break;
                case "progress": activity.setText(e.optString("value"));break;
                case "retry": new AlertDialog.Builder(app).setTitle(text("Transfer stopped")).setMessage(e.optString("value"))
                    .setNegativeButton(text("Cancel"),(d,w) -> run("filesControl","retry",false))
                    .setPositiveButton(text("Retry"),(d,w) -> run("filesControl","retry",true))
                    .setOnCancelListener(d -> run("filesControl","retry",false)).show();break;
                case "export": save(e.optString("name"),e.optString("path"),e.optBoolean("tree"));break;
                case "preview": if (reader != null) reader.receive(e); break;
                case "closed":
                    ready=false;
                    activity.setText(e.optString("value"));
                    if (reader != null) reader.disconnected(e.optString("value"));
                    break;
                case "error": case "transferDone": activity.setText(e.optString("value"));break;
                default: app.handle(e,"filesAnswer",""); break;
            }
        }
    }

    private void pick(boolean tree) {
        if(!ready||documentBusy)return;
        documentBusy=true;
        Intent request=tree ? new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE) :
            new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*");
        app.startActivityForResult(request,tree ? PICK_TREE : PICK);
    }

    private static String contentType(String name) {
        int dot=name.lastIndexOf('.');
        String type=dot<0 ? null : android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(
            name.substring(dot+1).toLowerCase(java.util.Locale.ROOT));
        return type==null ? "application/octet-stream" : type;
    }

    private void openSaved(Uri document,String name) {
        Intent view=new Intent(Intent.ACTION_VIEW).setDataAndType(document,contentType(name))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        view.setClipData(android.content.ClipData.newRawUri(name,document));
        try { app.startActivity(Intent.createChooser(view,text("Open with..."))); }
        catch (android.content.ActivityNotFoundException | SecurityException error) {
            app.toast(text("No app could open the saved file."));
        }
    }

    private void save(String name,String local,boolean tree) {
        export=local;
        exportName=name;
        documentBusy=true;
        Intent request=tree ? new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE) :
            new Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)
                .setType(contentType(name)).putExtra(Intent.EXTRA_TITLE,name);
        app.startActivityForResult(request,tree ? SAVE_TREE : CREATE);
    }

    void result(int request,int result,Intent intent) {
        boolean download=request==CREATE||request==SAVE_TREE;
        boolean tree=request==PICK_TREE||request==SAVE_TREE;
        if(!download&&request!=PICK&&request!=PICK_TREE)return;
        if(result!=Activity.RESULT_OK || intent==null || intent.getData()==null) {
            if (download) {
                app.call("filesRelease");
                chooseAppAfterSave=false;
            }
            documentBusy=false;
            return;
        }
        Uri document=intent.getData();
        if(download) {
            String source=export, name=exportName;
            boolean open=chooseAppAfterSave;
            chooseAppAfterSave=false;
            new Thread(() -> {
                String problem=null;
                try {
                    if (tree) new DocumentTree(app.getContentResolver(),document).exportFrom(java.nio.file.Paths.get(source),name);
                    else try(InputStream in=new FileInputStream(source);
                        OutputStream out=app.getContentResolver().openOutputStream(document,"wt")) { DocumentTree.copy(in,out); }
                } catch(Exception e){ problem=tree ? text("Could not save the folder. Check the selected location for copied files.") : text("Could not save the file."); }
                String error=problem;
                app.runOnUiThread(() -> {
                    app.call("filesRelease");
                    documentBusy=false;
                    activity.setText(error==null?text("Download saved"):error);
                    if (error==null && !tree && open && !app.isFinishing() && !app.privacyLocked())
                        openSaved(document,name);
                });
            }).start();
        } else {
            JSONObject target=app.call("filesTemp","tree",tree);
            if(error(target)){documentBusy=false;return;}
            String staged=target.optString("path");
            new Thread(() -> {
                String name="upload";
                try {
                    if(tree) name=new DocumentTree(app.getContentResolver(),document).importInto(java.nio.file.Paths.get(staged));
                    else {
                        try(android.database.Cursor cursor=app.getContentResolver().query(document,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){
                            if(cursor!=null&&cursor.moveToFirst()&&cursor.getString(0)!=null)name=cursor.getString(0);
                        } catch(RuntimeException ignored){}
                        try(InputStream in=app.getContentResolver().openInputStream(document);
                            OutputStream out=new FileOutputStream(staged)) { DocumentTree.copy(in,out); }
                    }
                } catch(Exception e) {
                    app.runOnUiThread(() -> {documentBusy=false; activity.setText(text("Could not read the selected item."));});
                    return;
                }
                String label=name;
                app.runOnUiThread(() -> {
                    documentBusy=false;
                    LinearLayout form=app.column();
                    EditText title=app.field(form,text("Remote name"),label,InputType.TYPE_CLASS_TEXT);
                    android.widget.CheckBox replace=app.check(form,text("Replace existing item"));
                    new AlertDialog.Builder(app).setTitle(tree ? text("Upload folder") : text("Upload file")).setView(form)
                        .setNegativeButton(text("Cancel"),null).setPositiveButton(text("Upload"),(d,w) ->
                            run("filesUpload","tree",tree,"newName",MainActivity.value(title),"replace",replace.isChecked())).show();
                });
            }).start();
        }
    }

}
