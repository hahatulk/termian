package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.AtomicFile;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.security.KeyStore;
import java.security.SecureRandom;
import java.util.Arrays;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

final class VaultKey {
    private static final String ALIAS = "app.termian.mobile.vault";
    private static final int IV_SIZE = 12;
    private static final int FILE_SIZE = 1 + IV_SIZE + 32 + 16;

    static byte[] load(File root) throws Exception {
        AtomicFile wrapped = new AtomicFile(new File(root, "vault.key"));
        KeyStore keys = KeyStore.getInstance("AndroidKeyStore");
        keys.load(null);
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        byte[] record;
        try {
            record = wrapped.readFully();
        } catch (java.io.FileNotFoundException e) {
            if (wrapped.getBaseFile().exists() || new File(root, "vault.bin").exists() || new File(root, "vaults.bin").exists()) {
                throw new IOException(text("Отсутствует ключ существующего Vault."), e);
            }
            return create(wrapped, keys, cipher);
        }
        if (record.length != FILE_SIZE || record[0] != 1 || !keys.containsAlias(ALIAS)) {
            throw new IOException(text("Не удалось прочитать ключ Vault."));
        }
        cipher.init(Cipher.DECRYPT_MODE, keys.getKey(ALIAS, null), new GCMParameterSpec(128, record, 1, IV_SIZE));
        return cipher.doFinal(record, 1 + IV_SIZE, record.length - 1 - IV_SIZE);
    }

    private static byte[] create(AtomicFile wrapped, KeyStore keys, Cipher cipher) throws Exception {
        if (!keys.containsAlias(ALIAS)) {
            KeyGenerator generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
            generator.init(new KeyGenParameterSpec.Builder(ALIAS,
                KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());
            generator.generateKey();
        }
        cipher.init(Cipher.ENCRYPT_MODE, (SecretKey) keys.getKey(ALIAS, null));
        byte[] raw = new byte[32];
        new SecureRandom().nextBytes(raw);
        try {
            byte[] iv = cipher.getIV();
            if (iv.length != IV_SIZE) throw new IOException(text("Неверный формат ключа Vault."));
            byte[] payload = cipher.doFinal(raw);
            FileOutputStream out = wrapped.startWrite();
            try {
                out.write(1);
                out.write(iv);
                out.write(payload);
                wrapped.finishWrite(out);
            } catch (Exception e) {
                wrapped.failWrite(out);
                throw e;
            }
            return raw;
        } catch (Exception e) {
            Arrays.fill(raw, (byte) 0);
            throw e;
        }
    }
}
