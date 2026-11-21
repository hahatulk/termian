package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.content.ContentResolver;
import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.HashSet;

final class DocumentTree {
    private static final String ID = DocumentsContract.Document.COLUMN_DOCUMENT_ID;
    private static final String NAME = DocumentsContract.Document.COLUMN_DISPLAY_NAME;
    private static final String MIME = DocumentsContract.Document.COLUMN_MIME_TYPE;
    private static final String DIR = DocumentsContract.Document.MIME_TYPE_DIR;
    private final ContentResolver resolver;
    private final Uri tree;
    private final HashSet<String> visited = new HashSet<>();
    private int remaining = 10000;

    DocumentTree(ContentResolver resolver, Uri tree) {
        this.resolver = resolver;
        this.tree = tree;
    }

    String importInto(Path local) throws IOException {
        String id = DocumentsContract.getTreeDocumentId(tree);
        Uri root = document(id);
        String label;
        try (Cursor cursor = resolver.query(root,new String[]{NAME},null,null,null)) {
            if (cursor == null || !cursor.moveToFirst()) throw new IOException();
            label = cursor.getString(0);
        }
        readChildren(id,local,0);
        return valid(label) ? label : text("Folder");
    }

    void exportFrom(Path local, String name) throws IOException {
        String id = DocumentsContract.getTreeDocumentId(tree);
        Cursor existing = children(id);
        try (Cursor cursor = existing) {
            while(cursor.moveToNext()) if (name.equals(cursor.getString(1))) throw new IOException(text("A folder with this name already exists."));
        }
        Uri target = create(document(id),DIR,name);
        writeChildren(local,target,0);
    }

    private Uri document(String id) { return DocumentsContract.buildDocumentUriUsingTree(tree,id); }

    private Cursor children(String id) throws IOException {
        Uri uri=DocumentsContract.buildChildDocumentsUriUsingTree(tree,id);
        Cursor result=resolver.query(uri,new String[]{ID,NAME,MIME},null,null,null);
        if(result==null)throw new IOException(text("Could not list the selected folder."));
        return result;
    }

    private Uri create(Uri parent,String mime,String name) throws IOException {
        if(!valid(name))throw new IOException(text("Invalid file name in folder."));
        Uri result=DocumentsContract.createDocument(resolver,parent,mime,name);
        if(result==null)throw new IOException(text("Could not create a document."));
        return result;
    }

    private static boolean valid(String name) {
        return name!=null&&!name.isEmpty()&&!name.equals(".")&&!name.equals("..")&&!name.contains("/")&&
            !name.contains("\\")&&name.indexOf('\0')<0;
    }

    private void count(int depth) throws IOException {
        if(depth>48||--remaining<0)throw new IOException(text("Folder exceeds the transfer limit."));
    }

    private void readChildren(String id,Path local,int depth) throws IOException {
        if(!visited.add(id))throw new IOException(text("The provider returned a repeated directory."));
        if(depth>48)throw new IOException(text("Folder exceeds the transfer limit."));
        try(Cursor cursor=children(id)) {
            while(cursor.moveToNext()) {
                count(depth);
                String child=cursor.getString(0), name=cursor.getString(1), mime=cursor.getString(2);
                if(!valid(name))throw new IOException(text("Invalid file name in folder."));
                Path target=local.resolve(name);
                if(DIR.equals(mime)) {
                    Files.createDirectory(target);
                    readChildren(child,target,depth+1);
                } else try(InputStream in=resolver.openInputStream(document(child));
                    OutputStream out=Files.newOutputStream(target,StandardOpenOption.CREATE_NEW,StandardOpenOption.WRITE)) {
                    copy(in,out);
                }
            }
        }
    }

    private void writeChildren(Path local,Uri target,int depth) throws IOException {
        try(DirectoryStream<Path> list=Files.newDirectoryStream(local)) {
            for(Path child:list) {
                count(depth);
                String name=child.getFileName().toString();
                if(Files.isDirectory(child,LinkOption.NOFOLLOW_LINKS)) {
                    writeChildren(child,create(target,DIR,name),depth+1);
                } else if(Files.isRegularFile(child,LinkOption.NOFOLLOW_LINKS)) {
                    Uri file=create(target,"application/octet-stream",name);
                    try(InputStream in=Files.newInputStream(child);OutputStream out=resolver.openOutputStream(file,"wt")) {
                        copy(in,out);
                    }
                }
            }
        }
    }

    static void copy(InputStream in,OutputStream out) throws IOException {
        if(in==null||out==null)throw new IOException(text("Could not open the document."));
        byte[] block=new byte[65536];
        for(int n;(n=in.read(block))!=-1;)out.write(block,0,n);
    }
}
