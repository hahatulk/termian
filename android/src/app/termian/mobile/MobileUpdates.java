package app.termian.mobile;

import android.app.AlertDialog;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.net.Uri;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.provider.Settings;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.TextView;
import java.io.File;
import java.io.IOException;
import java.util.Arrays;
import java.util.HashSet;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONObject;

final class MobileUpdates {
    private static final long CHECK_INTERVAL=6*60*60*1000L;
    private final MainActivity app;
    private final SharedPreferences preferences;
    private final Handler handler=new Handler(Looper.getMainLooper());
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private final Runnable scheduled=this::check;
    private final boolean enabled=nativeEnabled();
    private long nextCheck=SystemClock.elapsedRealtime()+20000L;
    private boolean visible, closed, busy, prerelease;
    private File readyApk;
    private String message;
    private TextView status;
    private ProgressBar progress;
    private RadioGroup channels;
    private AlertDialog dialog;
    private int percent=-1;

    MobileUpdates(MainActivity app) {
        this.app=app;
        preferences=app.getPreferences(Context.MODE_PRIVATE);
        prerelease=preferences.getBoolean("updates:preRelease",false);
        message=enabled ? "Checks run automatically while Termian is open." : "Updates are disabled in this build.";
    }

    private static native boolean nativeEnabled();
    private native String nativeUpdate(boolean prerelease,String destination);

    void start() {
        visible=true;
        schedule();
    }

    void stop() {
        visible=false;
        handler.removeCallbacks(scheduled);
    }

    void close() {
        closed=true;
        handler.removeCallbacksAndMessages(null);
        worker.shutdown();
        dismiss();
    }

    void dismiss() {
        if(dialog!=null) dialog.dismiss();
    }

    private void schedule() {
        handler.removeCallbacks(scheduled);
        if(enabled && visible && !closed && !busy && readyApk==null)
            handler.postDelayed(scheduled,Math.max(0,nextCheck-SystemClock.elapsedRealtime()));
    }

    void show() {
        if(dialog!=null) return;
        LinearLayout form=app.column();
        String version="";
        try { version=app.getPackageManager().getPackageInfo(app.getPackageName(),0).versionName; }
        catch(PackageManager.NameNotFoundException ignored) {}
        form.addView(app.label("Termian "+version,18,TermianStyle.TEXT));
        form.addView(app.label("Update channel",14,TermianStyle.TEXT));
        channels=new RadioGroup(app);
        RadioButton latest=new RadioButton(app);
        latest.setId(View.generateViewId());
        latest.setText("Latest");
        channels.addView(latest);
        RadioButton preview=new RadioButton(app);
        preview.setId(View.generateViewId());
        preview.setText("Pre-release");
        channels.addView(preview);
        channels.check(prerelease ? preview.getId() : latest.getId());
        channels.setOnCheckedChangeListener((group,id) -> selectChannel(id==preview.getId()));
        form.addView(channels);
        status=app.label("",14,TermianStyle.MUTED);
        form.addView(status);
        progress=new ProgressBar(app,null,android.R.attr.progressBarStyleHorizontal);
        progress.setMax(100);
        form.addView(progress);

        dialog=new AlertDialog.Builder(app).setTitle("Version").setView(app.scroll(form))
            .setNegativeButton("Close",null).setPositiveButton("Check for updates",null).show();
        dialog.setOnDismissListener(d -> dialog=null);
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(v -> {
            if(readyApk==null) check();
            else install();
        });
        render();
    }

    private void selectChannel(boolean preview) {
        if(busy || prerelease==preview) return;
        prerelease=preview;
        preferences.edit().putBoolean("updates:preRelease",prerelease).apply();
        if(readyApk!=null) {
            app.revokeUriPermission(UpdateApkProvider.uri(app,readyApk),Intent.FLAG_GRANT_READ_URI_PERMISSION);
            readyApk.delete();
            readyApk=null;
        }
        message=enabled ? "Update channel changed. Check for updates." : "Updates are disabled in this build.";
        nextCheck=SystemClock.elapsedRealtime()+CHECK_INTERVAL;
        render();
        schedule();
    }

    private void check() {
        if(!enabled || closed || busy || readyApk!=null) return;
        handler.removeCallbacks(scheduled);
        busy=true;
        message="Checking for updates…";
        percent=-1;
        render();
        boolean preview=prerelease;
        worker.execute(() -> {
            File apk=UpdateApkProvider.newApk(app);
            boolean verified=false;
            String result;
            try {
                File parent=apk.getParentFile();
                if(!parent.isDirectory() && !parent.mkdirs()) throw new IOException("Cannot create the update cache.");
                JSONObject reply=new JSONObject(nativeUpdate(preview,apk.getPath()));
                if(reply.has("error")) throw new IOException(reply.getString("error"));
                if(reply.has("version")) {
                    String candidate=reply.getString("version");
                    verifyApk(apk,candidate);
                    verified=true;
                    result="Termian "+candidate+" is ready to install. Installation closes open sessions.";
                } else result="No newer version in this channel.";
            } catch(Exception error) {
                result=error.getMessage()==null ? "Update failed." : error.getMessage();
            }
            final String done=result;
            final File downloaded=verified ? apk : null;
            if(!verified) apk.delete();
            handler.post(() -> {
                if(closed) {
                    if(downloaded!=null) downloaded.delete();
                    return;
                }
                busy=false;
                readyApk=downloaded;
                message=done;
                if(downloaded!=null && visible && dialog==null && !app.privacyLocked())
                    app.toast(done+" Open Settings > Version.");
                nextCheck=SystemClock.elapsedRealtime()+CHECK_INTERVAL;
                render();
                schedule();
            });
        });
    }

    private void updateProgress(String version,long downloaded,long total) {
        handler.post(() -> {
            if(closed || !busy) return;
            message="Downloading Termian "+version+"… "+(downloaded/1048576)+" MB";
            percent=total>0 ? (int)Math.min(100,downloaded*100/total) : -1;
            render();
        });
    }

    private void verifyApk(File apk,String version) throws Exception {
        PackageManager manager=app.getPackageManager();
        int flags=Build.VERSION.SDK_INT>=28 ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
        PackageInfo current=manager.getPackageInfo(app.getPackageName(),flags);
        PackageInfo update=manager.getPackageArchiveInfo(apk.getPath(),flags);
        if(update==null || !app.getPackageName().equals(update.packageName) || !version.equals(update.versionName))
            throw new IOException("The APK does not match the selected Termian release.");
        long installed=Build.VERSION.SDK_INT>=28 ? current.getLongVersionCode() : current.versionCode;
        long incoming=Build.VERSION.SDK_INT>=28 ? update.getLongVersionCode() : update.versionCode;
        if(incoming<=installed) throw new IOException("The APK version code is not newer than the installed version.");
        Signature[] currentSigners=signers(current), updateSigners=signers(update);
        if(currentSigners==null || currentSigners.length==0 || updateSigners==null ||
            !new HashSet<>(Arrays.asList(currentSigners)).equals(new HashSet<>(Arrays.asList(updateSigners))))
            throw new IOException("The APK signing certificate does not match the installed app.");
    }

    private Signature[] signers(PackageInfo info) {
        if(Build.VERSION.SDK_INT<28) return info.signatures;
        return info.signingInfo==null ? null : info.signingInfo.getApkContentsSigners();
    }

    private void install() {
        if(readyApk==null) return;
        if(!readyApk.isFile()) {
            readyApk=null;
            check();
            return;
        }
        try {
            if(!app.getPackageManager().canRequestPackageInstalls()) {
                message="Allow installs from Termian in system settings, then return and tap Install.";
                render();
                app.startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.fromParts("package",app.getPackageName(),null)));
                return;
            }
            Intent intent=new Intent(Intent.ACTION_VIEW)
                .setDataAndType(UpdateApkProvider.uri(app,readyApk),UpdateApkProvider.MIME)
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            app.startActivity(intent);
        } catch(android.content.ActivityNotFoundException | SecurityException e) {
            message="Android could not open the package installer.";
            render();
        }
    }

    private void render() {
        if(dialog==null) return;
        status.setText(message);
        for(int i=0;i<channels.getChildCount();i++) channels.getChildAt(i).setEnabled(!busy);
        progress.setVisibility(busy ? View.VISIBLE : View.GONE);
        progress.setIndeterminate(percent<0);
        if(percent>=0) progress.setProgress(percent);
        Button action=dialog.getButton(AlertDialog.BUTTON_POSITIVE);
        action.setEnabled(enabled && !busy);
        action.setText(readyApk==null ? "Check for updates" : "Install");
    }
}
