// Copyright 2023-2025 The Android Open Source Project

use std::{env, path::PathBuf};

/// Returns the appropriate runtime directory for the current platform.
///
/// This logic is borrowed from `netsim-common` to ensure consistent
/// temporary directory handling that works in cloud test environments.
///
/// Under Forge temp directory is `$ANDROID_TMP/android-$USER/netsim-next`,
/// otherwise it is `$TMP/android-$USER/netsim-next`
pub fn get_runtime_dir() -> PathBuf {
    // allow Forge to override the system temp
    let mut path = match env::var("ANDROID_TMP") {
        Ok(var) => PathBuf::from(var),
        _ => env::temp_dir(),
    };
    // On Windows the GetTempPath() is user-dependent so we don't need
    // to append $USER to the result -- otherwise allow multiple users
    // to co-exist on a system.
    #[cfg(not(target_os = "windows"))]
    {
        let user = match env::var("USER") {
            Ok(var) => format!("android-{var}"),
            _ => "android".to_string(),
        };
        path.push(user);
    };
    // netsim-next files are stored in their own directory
    path.push("netsim-next");

    if !path.is_dir() {
        // It's okay to panic here. If we can't create the directory, we can't continue.
        std::fs::create_dir_all(&path).expect("Failed to create runtime directory for netsim-next");
    }
    path
}
