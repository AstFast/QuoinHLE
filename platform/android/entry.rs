// Android entry template. This file is not compiled by the desktop build.
//
// Wiring requires cargo-apk, the `android-activity` crate, and a `[package.metadata.android]`
// section in Cargo.toml. `manager::run` is the same native entry the desktop binary uses; the
// Android activity must point at this function through the NativeActivity manifest.

#![cfg(target_os = "android")]

use jni::objects::{JObject, JValue};
use jni::{jni_sig, jni_str};

/// Result code for the media-permission request. The caller is the native activity; this template
/// does not depend on `onRequestPermissionsResult`, but Android still requires a stable code.
const REQUEST_MEDIA: i32 = 0x5100;

/// Ask for the permissions needed by an external SoundFont or icon picker.
///
/// `READ_MEDIA_AUDIO`/`READ_MEDIA_IMAGES` are Android 13+; `READ_EXTERNAL_STORAGE` covers Android
/// 12 and below. All three are declared in the manifest, and Android ignores the ones that do not
/// apply to its version. App-private files under `internal_data_path()` need no grant, so this is
/// only about user-picked files.
///
/// The request is asynchronous: the activity shows the system dialog and records the answer. A
/// picker that needs a definitive answer should check the result through its own activity callback;
/// this helper deliberately does not block the manager while the dialog is up.
fn request_media_permissions(app: &android_activity::AndroidApp) {
    // The pointers are owned by `AndroidApp`; `JavaVM::from_raw` and the global activity reference
    // are valid for this call. Nothing is deleted here.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let activity = app.activity_as_ptr() as jni::sys::jobject;
    let attached = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = unsafe { JObject::from_raw(env, activity) };
        let permissions = [
            "android.permission.READ_MEDIA_AUDIO",
            "android.permission.READ_MEDIA_IMAGES",
            "android.permission.READ_EXTERNAL_STORAGE",
        ];
        let array = env.new_object_array(
            permissions.len() as jni::sys::jsize,
            "java/lang/String",
            JObject::null(),
        )?;
        for (index, permission) in permissions.iter().enumerate() {
            let text = env.new_string(*permission)?;
            env.set_object_array_element(&array, index, &text)?;
        }
        env.call_method(
            &activity,
            jni_str!("requestPermissions"),
            jni_sig!("([Ljava/lang/String;I)V"),
            &[JValue::Object(array.as_ref()), JValue::Int(REQUEST_MEDIA)],
        )?;
        Ok(())
    });
    if let Err(error) = attached {
        eprintln!("quoin: cannot request media permissions: {error}");
    }
}

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    // The app's private files directory; never a desktop-style path.
    let data = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    quoin::platform::set_data_dir(data.join("quoin-data"));
    quoin::platform::set_ui_lang("en");

    // The default template asks before the manager opens. If the prompt on first launch is not
    // wanted, move this call to the action that opens an external SoundFont/icon picker.
    request_media_permissions(&app);

    // Put the packs below the same private directory, or copy them there from assets.
    let root = data.join("Game");
    if let Err(e) = quoin::manager::run(&root, "graphite", None) {
        eprintln!("quoin: {e}");
    }
}
