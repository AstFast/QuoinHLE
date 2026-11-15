//! The system/environment language, before the manager maps it to its two UI languages.

/// Return the first `zh*` or `en*` tag the environment or OS gives. `None` means the caller's
/// default (English). The order is deliberate: explicit environment variables beat system settings.
pub fn system_tag() -> Option<String> {
    for key in ["QUOIN_UI_LANG", "LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE"] {
        let Ok(tag) = std::env::var(key) else {
            continue;
        };
        let tag = tag.to_ascii_lowercase();
        if tag.starts_with("zh") || tag.starts_with("en") {
            return Some(tag);
        }
    }
    #[cfg(target_os = "macos")]
    {
        // A desktop launch often has no `LANG`; the system preference is the only answer there.
        if let Ok(out) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLocale"])
            .output()
        {
            if String::from_utf8_lossy(&out.stdout)
                .to_ascii_lowercase()
                .starts_with("zh")
            {
                return Some("zh".into());
            }
        }
    }
    #[cfg(windows)]
    {
        // `GetUserDefaultUILanguage` returns a LANGID; the low ten bits are the primary language,
        // and `LANG_CHINESE` is 0x04.
        if unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() } & 0x3ff == 0x04
        {
            return Some("zh".into());
        }
    }
    None
}
