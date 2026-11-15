//! Console setup that differs per OS.

/// Ask the console to read our bytes as UTF-8.
///
/// Every report line is UTF-8 and a Windows console starts in codepage 936, so bytes the host meant
/// as one glyph were decoded as another. A no-op when stdout is a redirect or a pipe.
pub fn utf8_console() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetConsoleOutputCP(codepage: u32) -> i32;
        }
        // Safe either way: the call only reports whether the console took it.
        let _ = unsafe { SetConsoleOutputCP(65001) };
    }
}
