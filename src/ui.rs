//! Terminal setup, color styling, and human-readable sizes.

/// Colored-output helper; all methods are no-ops when color is disabled.
pub struct Ui {
    color: bool,
}

impl Ui {
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    fn paint(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    pub fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }
    pub fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }
    pub fn green(&self, s: &str) -> String {
        self.paint("32", s)
    }
    pub fn yellow(&self, s: &str) -> String {
        self.paint("33", s)
    }
    pub fn red(&self, s: &str) -> String {
        self.paint("31", s)
    }
    pub fn cyan(&self, s: &str) -> String {
        self.paint("36", s)
    }

    /// Cyan text right-aligned in `width` columns (size column).
    pub fn cyan_right(&self, width: usize, s: &str) -> String {
        self.cyan(&format!("{s:>width$}"))
    }
}

/// Format a byte count as `1.23 GB` / `4.5 MB` / `12.0 KB` / `987 B`.
pub fn fmt_size(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Enable UTF-8 output + virtual-terminal escape codes in the console.
/// Returns false when the console does not support them.
#[cfg(windows)]
pub fn init_terminal() -> bool {
    extern "system" {
        fn GetStdHandle(nstdhandle: i32) -> *mut core::ffi::c_void;
        fn GetConsoleMode(hconsolehandle: *mut core::ffi::c_void, lpmode: *mut u32) -> i32;
        fn SetConsoleMode(hconsolehandle: *mut core::ffi::c_void, dwmode: u32) -> i32;
        fn SetConsoleOutputCP(wcodepageid: u32) -> i32;
    }
    const STD_OUTPUT_HANDLE: i32 = -11;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    unsafe {
        SetConsoleOutputCP(65001);
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle as isize == -1 {
            return false;
        }
        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

#[cfg(not(windows))]
pub fn init_terminal() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes_at_each_unit() {
        assert_eq!(fmt_size(0), "0 B");
        assert_eq!(fmt_size(512), "512 B");
        assert_eq!(fmt_size(2048), "2.0 KB");
        assert_eq!(fmt_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(fmt_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn color_can_be_disabled() {
        let plain = Ui::new(false);
        assert_eq!(plain.red("boom"), "boom");
        let colored = Ui::new(true);
        assert!(colored.red("boom").contains("boom"));
    }
}
