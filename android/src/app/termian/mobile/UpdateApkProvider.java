package app.termian.mobile;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import java.io.File;
import java.io.FileNotFoundException;

public final class UpdateApkProvider extends ContentProvider {
    static final String MIME = "application/vnd.android.package-archive";

    static File newApk(Context context) {
        File directory=new File(context.getCacheDir(),"updates");
        File[] previous=directory.listFiles();
        long cutoff=System.currentTimeMillis()-24*60*60*1000L;
        if(previous!=null) for(File file : previous) {
            if(isApkName(file.getName()) && file.lastModified()<cutoff) file.delete();
        }
        return new File(directory,"termian-"+java.util.UUID.randomUUID()+".apk");
    }

    static Uri uri(Context context,File apk) {
        return Uri.parse("content://"+context.getPackageName()+".updates/"+apk.getName());
    }

    private static boolean isApkName(String name) {
        return name!=null && name.matches("termian-[0-9a-f-]{36}\\.apk");
    }

    private File find(Uri uri) {
        String name=uri.getLastPathSegment();
        if(!isApkName(name)) return null;
        File apk=new File(new File(getContext().getCacheDir(),"updates"),name);
        return uri.equals(uri(getContext(),apk)) && apk.isFile() ? apk : null;
    }

    @Override public boolean onCreate() { return true; }

    @Override public String getType(Uri uri) { return find(uri)==null ? null : MIME; }

    @Override public ParcelFileDescriptor openFile(Uri uri,String mode) throws FileNotFoundException {
        File apk=find(uri);
        if(!"r".equals(mode) || apk==null) throw new FileNotFoundException();
        return ParcelFileDescriptor.open(apk,ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override public Cursor query(Uri uri,String[] projection,String selection,String[] args,String sort) {
        File apk=find(uri);
        if(apk==null) return null;
        String[] columns=projection==null ? new String[]{OpenableColumns.DISPLAY_NAME,OpenableColumns.SIZE} : projection;
        MatrixCursor cursor=new MatrixCursor(columns,1);
        Object[] row=new Object[columns.length];
        for(int i=0;i<columns.length;i++) {
            if(OpenableColumns.DISPLAY_NAME.equals(columns[i])) row[i]="termian.apk";
            if(OpenableColumns.SIZE.equals(columns[i])) row[i]=apk.length();
        }
        cursor.addRow(row);
        return cursor;
    }

    @Override public Uri insert(Uri uri,ContentValues values) { throw new UnsupportedOperationException(); }
    @Override public int update(Uri uri,ContentValues values,String where,String[] args) { throw new UnsupportedOperationException(); }
    @Override public int delete(Uri uri,String where,String[] args) { throw new UnsupportedOperationException(); }
}
