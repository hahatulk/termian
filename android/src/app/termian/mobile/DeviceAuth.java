package app.termian.mobile;

import static app.termian.mobile.MainActivity.text;

import android.app.Activity;
import android.app.KeyguardManager;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.hardware.biometrics.BiometricManager.Authenticators;
import android.hardware.biometrics.BiometricPrompt;
import android.os.Build;
import android.os.Bundle;
import android.os.CancellationSignal;
import java.util.concurrent.Executor;

final class DeviceAuth {
    private static final int CONFIRM_SCREEN_LOCK = 12;
    private static final String CONFIRMING = "auth:confirming";
    private final MainActivity app;
    private final KeyguardManager keyguard;
    private CancellationSignal biometric;
    private boolean confirming;

    DeviceAuth(MainActivity app, Bundle state) {
        this.app = app;
        keyguard = app.getSystemService(KeyguardManager.class);
        confirming = state != null && state.getBoolean(CONFIRMING);
    }

    boolean secure() {
        return keyguard != null && keyguard.isDeviceSecure();
    }

    void save(Bundle state) {
        state.putBoolean(CONFIRMING, confirming);
    }

    void request() {
        if (!canStart()) return;
        if (Build.VERSION.SDK_INT < 28) {
            confirmScreenLock();
            return;
        }
        CancellationSignal signal = new CancellationSignal();
        biometric = signal;
        try {
            Api28.request(this, signal);
        } catch (SecurityException | IllegalStateException e) {
            useScreenLock(signal);
        }
    }

    private boolean canStart() {
        return biometric == null && !confirming && !app.isFinishing() && !app.isDestroyed() && secure();
    }

    private void useScreenLock(CancellationSignal signal) {
        if (biometric != signal) return;
        cancelBiometric();
        if (secure()) confirmScreenLock();
        else app.authenticationUnavailable();
    }

    private void confirmScreenLock() {
        if (!canStart()) return;
        try {
            Intent intent = keyguard.createConfirmDeviceCredentialIntent(text("Unlock Termian"), null);
            if (intent != null) {
                confirming = true;
                app.startActivityForResult(intent, CONFIRM_SCREEN_LOCK);
                return;
            }
        } catch (ActivityNotFoundException | SecurityException e) {
            confirming = false;
        }
        app.authenticationUnavailable();
    }

    boolean result(int request, int result) {
        if (request != CONFIRM_SCREEN_LOCK) return false;
        if (confirming) {
            confirming = false;
            if (result == Activity.RESULT_OK) app.deviceAuthenticated();
        }
        return true;
    }

    void cancelBiometric() {
        CancellationSignal signal = biometric;
        biometric = null;
        if (signal != null) signal.cancel();
    }

    private static final class Api28 {
        static void request(DeviceAuth auth, CancellationSignal signal) {
            Executor main = auth.app.getMainExecutor();
            BiometricPrompt.Builder builder = new BiometricPrompt.Builder(auth.app).setTitle(text("Unlock Termian"));
            if (Build.VERSION.SDK_INT >= 30) {
                builder.setAllowedAuthenticators(Authenticators.BIOMETRIC_STRONG | Authenticators.DEVICE_CREDENTIAL);
            } else {
                builder.setNegativeButton(text("Use screen lock"), main, (dialog, which) -> auth.useScreenLock(signal));
            }
            builder.build().authenticate(signal, main, new BiometricPrompt.AuthenticationCallback() {
                @Override public void onAuthenticationSucceeded(BiometricPrompt.AuthenticationResult result) {
                    if (auth.biometric != signal) return;
                    auth.biometric = null;
                    auth.app.deviceAuthenticated();
                }

                @Override public void onAuthenticationError(int code, CharSequence message) {
                    if (auth.biometric != signal) return;
                    if (code == BiometricPrompt.BIOMETRIC_ERROR_CANCELED || code == BiometricPrompt.BIOMETRIC_ERROR_USER_CANCELED) {
                        auth.biometric = null;
                    } else {
                        auth.useScreenLock(signal);
                    }
                }
            });
        }
    }
}
