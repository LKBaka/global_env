use std::env;

/// 永久 新增/覆盖 一个环境变量，立即对当前进程也生效
pub fn set_global_env(key: &str, value: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?
            .set_value(key, &value)?;

        // 通知资源管理器环境变量已更改
        use std::ptr::null_mut;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
        };

        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(
                    "Environment\0"
                        .encode_utf16()
                        .collect::<Vec<u16>>()
                        .as_ptr() as isize,
                ),
                SMTO_ABORTIFHUNG,
                5000,
                Some(null_mut()),
            );
        }
    }

    #[cfg(unix)]
    {
        use std::{fs::OpenOptions, io::Write};

        let mut path = env::home_dir().expect("no home");
        path.push(".profile");
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(f, r#"export {key}="{value}""#)?;
    }

    // 让当前进程立即生效
    unsafe { env::set_var(key, value) };
    Ok(())
}