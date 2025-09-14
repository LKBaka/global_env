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

/// 获取全局环境变量（包括注册表中的）
pub fn get_global_env(key: &str) -> Option<String> {
    // 先尝试当前进程
    if let Ok(val) = env::var(key) {
        return Some(val);
    }

    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};
        
        // 尝试用户环境变量
        if let Ok(env_key) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Environment") 
        {
            if let Ok(val) = env_key.get_value(key) {
                return Some(val);
            }
        }
        
        // 尝试系统环境变量
        if let Ok(env_key) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment") 
        {
            if let Ok(val) = env_key.get_value(key) {
                return Some(val);
            }
        }
    }

    #[cfg(unix)]
    get_global_env_unix(key);

    None
}

#[cfg(unix)]
fn get_shell_config_file(shell: &str) -> String {
    match shell {
        "bash" => ".bashrc".to_string(),
        "zsh" => ".zshrc".to_string(),
        "fish" => ".config/fish/config.fish".to_string(),
        "ksh" => ".kshrc".to_string(),
        "tcsh" => ".tcshrc".to_string(),
        "csh" => ".cshrc".to_string(),
        _ => ".profile".to_string(), // 默认使用.profile
    }
}

#[cfg(unix)]
/// 获取全局环境变量（包括配置文件中的）
pub fn get_global_env_unix(key: &str) -> Option<String> {
    // 先尝试当前进程
    if let Ok(val) = env::var(key) {
        return Some(val);
    }

    // 尝试从配置文件中读取
    let shell = env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let config_file = get_shell_config_file(&shell);

    if let Some(home) = env::var_os("HOME") {
        use std::path::Path;
        let mut path = Path::new(&home).to_path_buf();
        path.push(&config_file);
        
        if let Ok(content) = std::fs::read_to_string(&path) {
            let pattern = format!("export {}=", key);
            for line in content.lines() {
                if line.starts_with(&pattern) {
                    if let Some(value) = line.splitn(2, '=').nth(1) {
                        return Some(value.trim_matches('"').to_string());
                    }
                }
            }
        }
    }

    None
}