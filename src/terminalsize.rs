// Terminal size detection for Slowfetch.
// a lot of this code is from stack overflow.

use std::os::unix::io::AsRawFd;

//tells Rust to use c-compatible memory layout
//need this because im interfacing with the kernel's ioctl syscall
#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

// TIOCGWINSZ constant for Linux
const TIOCGWINSZ: u64 = 0x5413;

// Get the terminal size as, columns and rows
// Returns None if the terminal size cannot be determined.
pub fn get_terminal_size() -> Option<(u16, u16)> {
    use std::io::stdout;

    unsafe {
        //uhoh
        let mut ws = std::mem::MaybeUninit::<Winsize>::zeroed();
        let fd = stdout().as_raw_fd();

        #[cfg(target_os = "linux")]
        {
            let result = libc_ioctl(fd, TIOCGWINSZ, ws.as_mut_ptr());
            if result == 0 {
                let ws = ws.assume_init();
                if ws.ws_col > 0 && ws.ws_row > 0 {
                    return Some((ws.ws_col, ws.ws_row));
                }
            }
        }
    }

    // Fallback to environment variables
    get_size_from_env()
}

#[cfg(target_os = "linux")]
unsafe fn libc_ioctl(fd: i32, request: u64, winsize: *mut Winsize) -> i32 {
    // Direct syscall
    let result: i64;
    unsafe {
        std::arch::asm!(
            "syscall",
            in("rax") 16, // SYS_ioctl
            in("rdi") fd,
            in("rsi") request,
            in("rdx") winsize,
            lateout("rax") result,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    result as i32
}

fn get_size_from_env() -> Option<(u16, u16)> {
    let cols = std::env::var("COLUMNS").ok()?.parse().ok()?;
    let rows = std::env::var("LINES").ok()?.parse().ok()?;
    Some((cols, rows))
}
