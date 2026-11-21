package app.termian.mobile;

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
    private String export, exportName;
    private boolean ready, documentBusy;
    boolean polling;

    MobileFiles(MainActivity app) {
        this.app = app;
        view=app.column();
        remote=app.column();
        refreshChooser();
    }

    void arrange(boolean wide) {
        view.removeAllViews();
        view.setOrientation(wide ? LinearLayout.HORIZONTAL : LinearLayout.VERTICAL);
        if(wide) {
            LinearLayout local=app.column();
            local.setBackgroundColor(TermianStyle.SIDEBAR);
            local.setPadding(app.dp(12),app.dp(8),app.dp(12),app.dp(8));
            local.addView(app.label("Local",18,TermianStyle.TEXT));
            local.addView(app.label("Choose Android files or folders to upload.",13,TermianStyle.MUTED));
            local.addView(app.style.tool("upload","SELECT FILE",TermianStyle.SURFACE,() -> chooseLocal(false)));
            local.addView(app.style.tool("group","SELECT FOLDER",TermianStyle.SURFACE,() -> chooseLocal(true)));
            view.addView(local,new LinearLayout.LayoutParams(0,-1,1));
            View divider=new View(app);
            divider.setBackgroundColor(TermianStyle.BAR);
            view.addView(divider,new LinearLayout.LayoutParams(app.dp(1),-1));
        }
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(wide ? 0 : -1,wide ? -1 : 0,1);
        view.addView(remote,lp);
    }

    private void chooseLocal(boolean tree) {
        if(!ready) { app.toast("Connect to an SSH host first."); return; }
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
        remote.addView(app.label("Connect to host",18,TermianStyle.TEXT));
        remote.addView(app.label("Select a saved host to browse its files.",13,TermianStyle.MUTED));
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
            new AlertDialog.Builder(app).setTitle("Switch SFTP host?").setMessage("The current SFTP connection and its transfers will close.")
                .setNegativeButton("Cancel",null).setPositiveButton("Switch",(d,w) -> start(host)).show();
        } else start(host);
    }

    private void start(JSONObject host) {
        app.allowNetwork(() -> {
            JSONObject response=app.call("filesOpen","id",host.optString("id"));
            if(error(response))return;
            polling=true;
            ready=false;
            remote.removeAllViews();
            LinearLayout header=new LinearLayout(app);
            header.setGravity(android.view.Gravity.CENTER_VERTICAL);
            header.setBackgroundColor(TermianStyle.SIDEBAR);
            TextView name=app.label(host.optString("label"),16,TermianStyle.TEXT);
            name.setMaxLines(1);
            header.addView(name,new LinearLayout.LayoutParams(0,-2,1));
            header.addView(app.style.iconAction("upload","Upload",() -> new AlertDialog.Builder(app).setTitle("Upload")
                .setItems(new String[]{"File","Folder"},(d,w)->pick(w==1)).show()));
            header.addView(app.style.iconAction("close","Disconnect",this::close));
            remote.addView(header,new LinearLayout.LayoutParams(-1,app.dp(48)));
            activity=app.label("Connecting…",14,TermianStyle.MUTED);
            remote.addView(activity);
            path=app.label("/",14,TermianStyle.TEXT);
            path.setMaxLines(2);
            remote.addView(path);
            LinearLayout actions=new LinearLayout(app);
            actions.addView(app.style.iconAction("arrow_left","Back folder",() -> run("filesBack")));
            actions.addView(app.style.iconAction("arrow_right","Forward folder",() -> run("filesForward")));
            actions.addView(app.button("Up",() -> run("filesUp")),new LinearLayout.LayoutParams(0,app.dp(48),1));
            actions.addView(app.button("Refresh",() -> run("filesRefresh")),new LinearLayout.LayoutParams(0,app.dp(48),1));
            View more=app.style.iconAction("dots","File actions",() -> {});
            more.setOnClickListener(anchor -> {
                android.widget.PopupMenu menu=new android.widget.PopupMenu(app,anchor);
                menu.getMenu().add("New Folder").setOnMenuItemClickListener(v -> { folderDialog(null); return true; });
                menu.getMenu().add("Cancel transfer").setOnMenuItemClickListener(v -> { run("filesControl","retry",false); return true; });
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
        if (documentBusy) {app.toast("Finish the file picker or save operation first.");return;}
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
        if (!ready) { app.toast("Connect to an SFTP host first."); return; }
        if (documentBusy) { app.toast("Finish the current file operation first."); return; }
        error(app.call(operation,values));
    }

    private void folderDialog(JSONObject file) {
        LinearLayout form=app.column();
        EditText text=app.field(form,"Name",file==null?"":file.optString("name"),InputType.TYPE_CLASS_TEXT);
        new AlertDialog.Builder(app).setTitle(file==null?"New folder":"Rename").setView(form)
            .setNegativeButton("Cancel",null).setPositiveButton("Save",(d,w) -> {
                String name=MainActivity.value(text);
                if(file==null)run("filesMkdir","newName",name);
                else run("filesRename","name",file.optString("name"),"newName",name);
            }).show();
    }

    private void fileMenu(JSONObject file) {
        boolean folder=file.optBoolean("folder");
        boolean link=file.optBoolean("link");
        String name=file.optString("name");
        java.util.ArrayList<String> options=new java.util.ArrayList<>();
        if(!link) {
            if(folder) options.add("Open");
            options.add("Download");
        }
        options.add("Rename");
        if(!link) options.add("Permissions");
        options.add("Delete");
        new AlertDialog.Builder(app).setTitle(name).setItems(options.toArray(new String[0]),(d,w) -> {
            switch(options.get(w)) {
                case "Open": run("filesFolder","name",name);break;
                case "Download": run("filesDownload","name",name);break;
                case "Rename": folderDialog(file);break;
                case "Permissions":
                    LinearLayout form=app.column();
                    EditText mode=app.field(form,"Octal mode",MainActivity.fieldValue(file,"mode"),InputType.TYPE_CLASS_NUMBER);
                    new AlertDialog.Builder(app).setTitle("Permissions").setView(form)
                        .setNegativeButton("Cancel",null).setPositiveButton("Apply",(a,b) -> run("filesChmod","name",name,"mode",MainActivity.value(mode))).show();
                    break;
                case "Delete":
                    new AlertDialog.Builder(app).setTitle("Delete " + name + "?")
                        .setMessage(folder&&!link?"This deletes the folder and its contents.": "This deletes the remote entry.")
                        .setNegativeButton("Cancel",null).setPositiveButton("Delete",(a,b) -> run("filesDelete","name",name)).show();
                    break;
            }
        }).show();
    }

    void events(JSONArray events) {
        for(int i=0;i<events.length();i++) {
            JSONObject e=events.optJSONObject(i);
            switch(e.optString("type")) {
                case "connected": ready=true;activity.setText("Connected");break;
                case "listing":
                    path.setText(e.optString("path"));
                    rows.removeAllViews();
                    JSONArray entries=e.optJSONArray("entries");
                    for(int j=0;j<entries.length();j++) {
                        JSONObject f=entries.optJSONObject(j);
                        boolean dir=f.optBoolean("folder") && !f.optBoolean("link");
                        String name=f.optString("name");
                        add(rows,app.style.card(dir?"group":"file",TermianStyle.TILE,name,dir?"Folder":f.optString("size"),
                            () -> { if(dir)run("filesFolder","name",name);else fileMenu(f); },() -> fileMenu(f)));
                    }
                    if(entries.length()==0) rows.addView(app.label("Folder is empty",15,TermianStyle.MUTED));
                    break;
                case "progress": activity.setText(e.optString("value"));break;
                case "retry": new AlertDialog.Builder(app).setTitle("Transfer stopped").setMessage(e.optString("value"))
                    .setNegativeButton("Cancel",(d,w) -> run("filesControl","retry",false))
                    .setPositiveButton("Retry",(d,w) -> run("filesControl","retry",true))
                    .setOnCancelListener(d -> run("filesControl","retry",false)).show();break;
                case "export": save(e.optString("name"),e.optString("path"),e.optBoolean("tree"));break;
                case "closed": ready=false;activity.setText(e.optString("value"));break;
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

    private void save(String name,String local,boolean tree) {
        export=local;
        exportName=name;
        documentBusy=true;
        Intent request=tree ? new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE) :
            new Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)
                .setType("application/octet-stream").putExtra(Intent.EXTRA_TITLE,name);
        app.startActivityForResult(request,tree ? SAVE_TREE : CREATE);
    }

    void result(int request,int result,Intent intent) {
        boolean download=request==CREATE||request==SAVE_TREE;
        boolean tree=request==PICK_TREE||request==SAVE_TREE;
        if(!download&&request!=PICK&&request!=PICK_TREE)return;
        if(result!=Activity.RESULT_OK || intent==null || intent.getData()==null) {
            if(download)app.call("filesRelease");
            documentBusy=false;
            return;
        }
        Uri document=intent.getData();
        if(download) {
            String source=export, name=exportName;
            new Thread(() -> {
                String problem=null;
                try {
                    if (tree) new DocumentTree(app.getContentResolver(),document).exportFrom(java.nio.file.Paths.get(source),name);
                    else try(InputStream in=new FileInputStream(source);
                        OutputStream out=app.getContentResolver().openOutputStream(document,"wt")) { DocumentTree.copy(in,out); }
                } catch(Exception e){ problem=tree ? "Could not save the folder. Check the selected location for copied files." : "Could not save the file."; }
                String error=problem;
                app.runOnUiThread(() -> {
                    app.call("filesRelease");
                    documentBusy=false;
                    activity.setText(error==null?"Download saved":error);
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
                    app.runOnUiThread(() -> {documentBusy=false; activity.setText("Could not read the selected item.");});
                    return;
                }
                String label=name;
                app.runOnUiThread(() -> {
                    documentBusy=false;
                    LinearLayout form=app.column();
                    EditText title=app.field(form,"Remote name",label,InputType.TYPE_CLASS_TEXT);
                    android.widget.CheckBox replace=app.check(form,"Replace existing item");
                    new AlertDialog.Builder(app).setTitle(tree ? "Upload folder" : "Upload file").setView(form)
                        .setNegativeButton("Cancel",null).setPositiveButton("Upload",(d,w) ->
                            run("filesUpload","tree",tree,"newName",MainActivity.value(title),"replace",replace.isChecked())).show();
                });
            }).start();
        }
    }

}
