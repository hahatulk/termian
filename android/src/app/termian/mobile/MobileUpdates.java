package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

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
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.Spinner;
import android.widget.TextView;
import java.io.File;
import java.io.IOException;
import java.util.Arrays;
import java.util.HashSet;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.function.IntConsumer;
import org.json.JSONObject;

final class MobileUpdates {
    private static final long FIRST_CHECK=20_000L;
    private static final int[] INTERVAL_HOURS={1,6,12,24};
    private static final int DEFAULT_INTERVAL=1;

    private enum Mode {
        MANUAL("Off"),
        CHECK("Check only"),
        DOWNLOAD("Check and download");

        final String label;

        Mode(String label) { this.label=label; }
    }

    private final MainActivity app;
    private final SharedPreferences preferences;
    private final Handler handler=new Handler(Looper.getMainLooper());
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private final Runnable scheduled=this::automaticCheck;
    private final boolean enabled=nativeEnabled();
    private long nextCheck=SystemClock.elapsedRealtime()+FIRST_CHECK;
    private boolean visible, closed, busy, prerelease;
    private Mode mode;
    private int interval;
    private File readyApk;
    private String availableVersion, notifiedVersion, message;
    private TextView status;
    private ProgressBar progress;
    private RadioGroup channels;
    private Spinner modes, intervals;
    private Button checkButton, downloadButton;
    private int percent=-1;

    MobileUpdates(MainActivity app) {
        this.app=app;
        preferences=app.getPreferences(Context.MODE_PRIVATE);
        prerelease=preferences.getBoolean("updates:preRelease",false);
        try { mode=Mode.valueOf(preferences.getString("updates:mode",Mode.DOWNLOAD.name())); }
        catch(IllegalArgumentException ignored) { mode=Mode.DOWNLOAD; }
        int intervalIndex=Arrays.binarySearch(INTERVAL_HOURS,preferences.getInt("updates:intervalHours",6));
        interval=intervalIndex<0 ? DEFAULT_INTERVAL : intervalIndex;
        message=enabled ? text("Updates have not been checked yet.") : text("Updates are disabled in this build.");
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
        detach();
    }

    void detach() {
        status=null;
        progress=null;
        channels=null;
        modes=null;
        intervals=null;
        checkButton=null;
        downloadButton=null;
    }

    private void schedule() {
        handler.removeCallbacks(scheduled);
        if(enabled && visible && !closed && !busy && readyApk==null && mode!=Mode.MANUAL)
            handler.postDelayed(scheduled,Math.max(0,nextCheck-SystemClock.elapsedRealtime()));
    }

    private long intervalMillis() {
        return INTERVAL_HOURS[interval]*3_600_000L;
    }

    private void automaticCheck() {
        if(visible && mode!=Mode.MANUAL) check(mode==Mode.DOWNLOAD);
    }

    private Spinner choices(String[] labels,int selected,IntConsumer select) {
        Spinner spinner=new Spinner(app);
        ArrayAdapter<String> adapter=new ArrayAdapter<>(app,android.R.layout.simple_spinner_item,labels);
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        spinner.setAdapter(adapter);
        spinner.setSelection(selected);
        spinner.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            @Override public void onItemSelected(AdapterView<?> parent,View view,int position,long id) { select.accept(position); }
            @Override public void onNothingSelected(AdapterView<?> parent) {}
        });
        return spinner;
    }

    void build(LinearLayout form) {
        String version="";
        try { version=app.getPackageManager().getPackageInfo(app.getPackageName(),0).versionName; }
        catch(PackageManager.NameNotFoundException ignored) {}
        form.addView(app.label("Termian "+version,18,TermianStyle.TEXT));
        form.addView(app.label(text("Update channel"),14,TermianStyle.TEXT));
        channels=new RadioGroup(app);
        RadioButton latest=new RadioButton(app);
        latest.setId(View.generateViewId());
        latest.setText(text("Latest"));
        channels.addView(latest);
        RadioButton preview=new RadioButton(app);
        preview.setId(View.generateViewId());
        preview.setText(text("Pre-release"));
        channels.addView(preview);
        channels.check(prerelease ? preview.getId() : latest.getId());
        channels.setOnCheckedChangeListener((group,id) -> selectChannel(id==preview.getId()));
        form.addView(channels);
        form.addView(app.label(text("Automatic updates"),14,TermianStyle.TEXT));
        String[] modeLabels=Arrays.stream(Mode.values()).map(value -> text(value.label)).toArray(String[]::new);
        modes=choices(modeLabels,mode.ordinal(),value -> selectMode(Mode.values()[value]));
        form.addView(modes);
        form.addView(app.label(text("Check interval"),14,TermianStyle.TEXT));
        String[] intervalLabels=Arrays.stream(INTERVAL_HOURS).mapToObj(hours -> text(hours+(hours==1 ? " hour" : " hours"))).toArray(String[]::new);
        intervals=choices(intervalLabels,interval,this::selectInterval);
        form.addView(intervals);
        status=app.label("",14,TermianStyle.MUTED);
        form.addView(status);
        progress=new ProgressBar(app,null,android.R.attr.progressBarStyleHorizontal);
        progress.setMax(100);
        form.addView(progress);

        checkButton=app.button(text("Check for updates"),() -> check(false));
        downloadButton=app.button(text("Download update"),() -> {
            if(readyApk!=null) install();
            else if(availableVersion!=null) check(true);
        });
        form.addView(checkButton,new LinearLayout.LayoutParams(-1,-2));
        form.addView(downloadButton,new LinearLayout.LayoutParams(-1,-2));
        render();
    }

    private void selectMode(Mode next) {
        if(busy || mode==next) return;
        boolean wasManual=mode==Mode.MANUAL;
        mode=next;
        preferences.edit().putString("updates:mode",mode.name()).apply();
        nextCheck=SystemClock.elapsedRealtime()+(wasManual ? FIRST_CHECK : intervalMillis());
        render();
        schedule();
    }

    private void selectInterval(int next) {
        if(busy || mode==Mode.MANUAL || interval==next) return;
        interval=next;
        preferences.edit().putInt("updates:intervalHours",INTERVAL_HOURS[interval]).apply();
        nextCheck=SystemClock.elapsedRealtime()+intervalMillis();
        render();
        schedule();
    }

    private void selectChannel(boolean preview) {
        if(busy || readyApk!=null || prerelease==preview) return;
        prerelease=preview;
        preferences.edit().putBoolean("updates:preRelease",prerelease).apply();
        availableVersion=null;
        message=enabled ? text("Update channel changed. Check for updates.") : text("Updates are disabled in this build.");
        nextCheck=SystemClock.elapsedRealtime()+intervalMillis();
        render();
        schedule();
    }

    private void check(boolean download) {
        if(!enabled || closed || busy || readyApk!=null) return;
        handler.removeCallbacks(scheduled);
        busy=true;
        availableVersion=null;
        message=text("Checking for updates…");
        percent=-1;
        render();
        boolean preview=prerelease;
        worker.execute(() -> {
            File apk=download ? UpdateApkProvider.newApk(app) : null;
            boolean verified=false;
            String candidate=null;
            String result;
            try {
                if(apk!=null) {
                    File parent=apk.getParentFile();
                    if(!parent.isDirectory() && !parent.mkdirs()) throw new IOException(text("Cannot create the update cache."));
                }
                JSONObject reply=new JSONObject(nativeUpdate(preview,apk==null ? null : apk.getPath()));
                if(reply.has("error")) throw new IOException(reply.getString("error"));
                if(reply.has("version")) {
                    String version=reply.getString("version");
                    if(apk==null) {
                        candidate=version;
                        result="Termian "+version+" is available.";
                    } else {
                        verifyApk(apk,version);
                        verified=true;
                        result="Termian "+version+" is ready to install. Installation closes open sessions.";
                    }
                } else result=text("No newer version in this channel.");
            } catch(Exception error) {
                result=error.getMessage()==null ? text("Update failed.") : error.getMessage();
            }
            final String done=result;
            final File downloaded=verified ? apk : null;
            final String found=candidate;
            if(!verified && apk!=null) apk.delete();
            handler.post(() -> {
                if(closed) {
                    if(downloaded!=null) downloaded.delete();
                    return;
                }
                busy=false;
                readyApk=downloaded;
                availableVersion=found;
                message=done;
                if(visible && status==null && !app.privacyLocked()
                    && (downloaded!=null || (found!=null && !found.equals(notifiedVersion)))) {
                    notifiedVersion=found;
                    app.toast(done+" Open Settings > Version.");
                }
                nextCheck=SystemClock.elapsedRealtime()+intervalMillis();
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
            throw new IOException(text("The APK does not match the selected Termian release."));
        long installed=Build.VERSION.SDK_INT>=28 ? current.getLongVersionCode() : current.versionCode;
        long incoming=Build.VERSION.SDK_INT>=28 ? update.getLongVersionCode() : update.versionCode;
        if(incoming<=installed) throw new IOException(text("The APK version code is not newer than the installed version."));
        Signature[] currentSigners=signers(current), updateSigners=signers(update);
        if(currentSigners==null || currentSigners.length==0 || updateSigners==null ||
            !new HashSet<>(Arrays.asList(currentSigners)).equals(new HashSet<>(Arrays.asList(updateSigners))))
            throw new IOException(text("The APK signing certificate does not match the installed app."));
    }

    private Signature[] signers(PackageInfo info) {
        if(Build.VERSION.SDK_INT<28) return info.signatures;
        return info.signingInfo==null ? null : info.signingInfo.getApkContentsSigners();
    }

    private void install() {
        if(readyApk==null) return;
        if(!readyApk.isFile()) {
            readyApk=null;
            check(true);
            return;
        }
        try {
            if(!app.getPackageManager().canRequestPackageInstalls()) {
                message=text("Allow installs from Termian in system settings, then return and tap Install.");
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
            message=text("Android could not open the package installer.");
            render();
        }
    }

    private void render() {
        if(status==null) return;
        status.setText(text(message));
        boolean configurable=!busy && readyApk==null;
        for(int i=0;i<channels.getChildCount();i++) channels.getChildAt(i).setEnabled(configurable);
        modes.setEnabled(configurable);
        intervals.setEnabled(configurable && mode!=Mode.MANUAL);
        progress.setVisibility(busy ? View.VISIBLE : View.GONE);
        progress.setIndeterminate(percent<0);
        if(percent>=0) progress.setProgress(percent);
        checkButton.setEnabled(enabled && !busy && readyApk==null);
        downloadButton.setEnabled(enabled && !busy);
        downloadButton.setVisibility(readyApk!=null || availableVersion!=null ? View.VISIBLE : View.GONE);
        downloadButton.setText(readyApk!=null ? text("Install") : text("Download update"));
    }
}
