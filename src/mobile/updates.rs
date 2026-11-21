use jni::{
    objects::{JClass, JObject, JString, JValue},
    sys::{jboolean, jstring, JNI_FALSE, JNI_TRUE},
    JNIEnv,
};
use serde_json::json;
use std::path::Path;

use crate::updater::{self, Channel};

#[no_mangle]
pub extern "system" fn Java_app_termian_mobile_MobileUpdates_nativeEnabled(
    _: JNIEnv,
    _: JClass,
) -> jboolean {
    if updater::enabled() {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_app_termian_mobile_MobileUpdates_nativeUpdate(
    mut env: JNIEnv,
    view: JObject,
    prerelease: jboolean,
    destination: JString,
) -> jstring {
    let result = (|| {
        if !updater::enabled() {
            return Err("Автообновление отключено в этой сборке.".to_owned());
        }
        let channel = if prerelease == JNI_TRUE {
            Channel::PreRelease
        } else {
            Channel::Latest
        };
        let Some(package) = updater::check(&updater::current_version(), channel)? else {
            return Ok(json!({}));
        };
        if destination.is_null() {
            return Ok(json!({"version":package.version}));
        }
        let path = String::from(env.get_string(&destination).map_err(|e| e.to_string())?);
        let version = JObject::from(
            env.new_string(&package.version)
                .map_err(|e| e.to_string())?,
        );
        updater::download_apk(&package, Path::new(&path), |downloaded, total| {
            let args = [
                JValue::Object(&version),
                JValue::Long(downloaded as i64),
                JValue::Long(total.unwrap_or(0) as i64),
            ];
            if matches!(
                env.call_method(&view, "updateProgress", "(Ljava/lang/String;JJ)V", &args),
                Err(jni::errors::Error::JavaException)
            ) {
                let _ = env.exception_clear();
            }
        })?;
        Ok(json!({"version":package.version}))
    })()
    .unwrap_or_else(|error| json!({"error":error}));
    env.new_string(result.to_string())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
