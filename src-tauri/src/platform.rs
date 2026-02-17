use std::process::{Command, Stdio};

/// Ouvre une application par son nom
pub fn open_app(app_name: &str) {
    log::info!("[OPEN_APP] Opening application: {}", app_name);

    #[cfg(target_os = "macos")]
    {
        match Command::new("open")
            .args(["-a", app_name])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    log::info!("[OPEN_APP] Successfully opened {}", app_name);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::warn!("[OPEN_APP] Failed to open {}: {}", app_name, stderr);
                }
            }
            Err(e) => {
                log::error!("[OPEN_APP] Failed to execute open command: {}", e);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        match Command::new("cmd")
            .args(["/C", "start", "", app_name])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    log::info!("[OPEN_APP] Successfully opened {}", app_name);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::warn!("[OPEN_APP] Failed to open {}: {}", app_name, stderr);
                }
            }
            Err(e) => {
                log::error!("[OPEN_APP] Failed to execute start command: {}", e);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        match Command::new("xdg-open")
            .arg(app_name)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    log::info!("[OPEN_APP] Successfully opened {}", app_name);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::warn!("[OPEN_APP] Failed to open {}: {}", app_name, stderr);
                }
            }
            Err(e) => {
                log::error!("[OPEN_APP] Failed to execute xdg-open: {}", e);
            }
        }
    }
}

/// Tape du texte en utilisant le presse-papier (pour le streaming incrémental)
pub fn type_text_incremental(text: &str) {
    use std::io::Write;

    #[cfg(target_os = "macos")]
    {
        match Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
            Err(_) => return,
        }

        std::thread::sleep(std::time::Duration::from_millis(30));

        let script = r#"tell application "System Events" to keystroke "v" using command down"#;
        let _ = Command::new("osascript")
            .args(["-e", script])
            .output();
    }

    #[cfg(target_os = "windows")]
    {
        match Command::new("cmd")
            .args(["/C", "clip"])
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
            Err(_) => return,
        }

        std::thread::sleep(std::time::Duration::from_millis(30));

        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };
        let inputs: [INPUT; 4] = [
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_V, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_V, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 } } },
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 } } },
        ];
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    }

    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        if wayland {
            match Command::new("wl-copy")
                .stdin(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    let _ = child.wait();
                }
                Err(_) => return,
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
            let _ = Command::new("wtype").args(["-M", "ctrl", "v", "-m", "ctrl"]).output();
        } else {
            match Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    let _ = child.wait();
                }
                Err(_) => return,
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
            let _ = Command::new("xdotool").args(["key", "--clearmodifiers", "ctrl+v"]).output();
        }
    }
}

/// Colle le texte à la position du curseur
pub fn paste_text(text: &str) {
    log::debug!("[PASTE] paste_text called with: '{}'", &text[..text.len().min(50)]);

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;

        match Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    if let Err(e) = stdin.write_all(text.as_bytes()) {
                        log::error!("[PASTE] Failed to write to pbcopy stdin: {}", e);
                        return;
                    }
                }
                let _ = child.wait();
            }
            Err(e) => {
                log::error!("[PASTE] Failed to spawn pbcopy: {}", e);
                return;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        let script = r#"tell application "System Events" to keystroke "v" using command down"#;

        match Command::new("osascript")
            .args(["-e", script])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    log::info!("[PASTE] Text pasted successfully");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::warn!("[PASTE] AppleScript failed: {}", stderr);

                    let escaped_text = text.replace("\\", "\\\\").replace("\"", "\\\"");
                    let type_script = format!(
                        r#"tell application "System Events" to keystroke "{}""#,
                        escaped_text
                    );

                    match Command::new("osascript")
                        .args(["-e", &type_script])
                        .output()
                    {
                        Ok(output2) => {
                            if output2.status.success() {
                                log::info!("[PASTE] Text typed directly");
                            } else {
                                log::warn!("[PASTE] Text copied to clipboard. Use Cmd+V manually.");
                                log::warn!("[PASTE] Enable in: System Preferences > Privacy > Accessibility");
                            }
                        }
                        Err(e) => {
                            log::error!("[PASTE] Failed to execute type script: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("[PASTE] Failed to execute osascript: {}", e);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };

        use std::io::Write;
        match Command::new("cmd")
            .args(["/C", "clip"])
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    if let Err(e) = stdin.write_all(text.as_bytes()) {
                        log::error!("[PASTE] Failed to write to clip.exe stdin: {}", e);
                        return;
                    }
                }
                let _ = child.wait();
            }
            Err(e) => {
                log::error!("[PASTE] Failed to spawn clip.exe: {}", e);
                return;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        let inputs: [INPUT; 4] = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
                    },
                },
            },
        ];

        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };

        if sent == 4 {
            log::info!("[PASTE] Text pasted successfully");
        } else {
            log::warn!("[PASTE] SendInput failed: only {} of 4 inputs sent. Use Ctrl+V manually.", sent);
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;

        let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

        if wayland {
            match Command::new("wl-copy")
                .stdin(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        if let Err(e) = stdin.write_all(text.as_bytes()) {
                            log::error!("[PASTE] Failed to write to wl-copy: {}", e);
                            return;
                        }
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    log::error!("[PASTE] Failed to spawn wl-copy: {}. Install with 'sudo apt install wl-clipboard'", e);
                    return;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));

            match Command::new("wtype")
                .args(["-M", "ctrl", "v", "-m", "ctrl"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    log::info!("[PASTE] Text pasted via wtype");
                }
                _ => {
                    match Command::new("ydotool")
                        .args(["key", "29:1", "47:1", "47:0", "29:0"])
                        .output()
                    {
                        Ok(output) if output.status.success() => {
                            log::info!("[PASTE] Text pasted via ydotool");
                        }
                        _ => {
                            log::warn!("[PASTE] Text copied to clipboard. Use Ctrl+V manually.");
                        }
                    }
                }
            }
        } else {
            match Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        if let Err(e) = stdin.write_all(text.as_bytes()) {
                            log::error!("[PASTE] Failed to write to xclip: {}", e);
                            return;
                        }
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    log::error!("[PASTE] Failed to spawn xclip: {}. Install with 'sudo apt install xclip'", e);
                    return;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));

            match Command::new("xdotool")
                .args(["key", "--clearmodifiers", "ctrl+v"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    log::info!("[PASTE] Text pasted via xdotool");
                }
                Ok(_) | Err(_) => {
                    log::warn!("[PASTE] Text copied to clipboard. Use Ctrl+V manually.");
                }
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        log::warn!("[PASTE] Paste not implemented for this platform - text copied to clipboard");
    }
}

/// Simule Cmd+C (macOS) ou Ctrl+C (Windows/Linux) pour copier le texte sélectionné
pub fn copy_selected_text() {
    log::debug!("[COPY] Copying selected text to clipboard...");

    #[cfg(target_os = "macos")]
    {
        let script = r#"tell application "System Events" to keystroke "c" using command down"#;
        match Command::new("osascript")
            .args(["-e", script])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    log::debug!("[COPY] Cmd+C simulated successfully");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::warn!("[COPY] AppleScript Cmd+C failed: {}", stderr);
                }
            }
            Err(e) => {
                log::error!("[COPY] Failed to execute osascript for copy: {}", e);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_C,
        };

        let inputs: [INPUT; 4] = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_C, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_C, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
                    },
                },
            },
        ];

        unsafe {
            let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            if result == 4 {
                log::debug!("[COPY] Ctrl+C simulated via SendInput");
            } else {
                log::warn!("[COPY] SendInput returned {}, expected 4", result);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        if wayland {
            match Command::new("wtype")
                .args(["-M", "ctrl", "c", "-m", "ctrl"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    log::debug!("[COPY] Ctrl+C simulated via wtype");
                }
                _ => {
                    log::warn!("[COPY] wtype failed for Ctrl+C");
                }
            }
        } else {
            match Command::new("xdotool")
                .args(["key", "--clearmodifiers", "ctrl+c"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    log::debug!("[COPY] Ctrl+C simulated via xdotool");
                }
                _ => {
                    log::warn!("[COPY] xdotool failed for Ctrl+C");
                }
            }
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));
}

/// Set system volume (0-100)
pub fn set_volume(level: u8) {
    let level = level.min(100);
    log::info!("[VOLUME] Setting volume to {}%", level);

    #[cfg(target_os = "macos")]
    {
        let script = format!("set volume output volume {}", level);
        let _ = Command::new("osascript").args(["-e", &script]).output();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("powershell")
            .args(["-Command", &format!("(New-Object -ComObject WScript.Shell).SendKeys([char]173)")])
            .output();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", level)])
            .output();
    }
}

/// Toggle Do Not Disturb mode
pub fn toggle_dnd() {
    log::info!("[DND] Toggling Do Not Disturb");

    #[cfg(target_os = "macos")]
    {
        // Use Shortcuts app if available, fallback to notification center toggle
        let _ = Command::new("shortcuts")
            .args(["run", "Toggle Do Not Disturb"])
            .output()
            .or_else(|_| {
                Command::new("osascript")
                    .args(["-e", "tell application \"System Events\" to keystroke \"\" using {command down, shift down}"])
                    .output()
            });
    }

    #[cfg(target_os = "windows")]
    {
        log::info!("[DND] Do Not Disturb toggle not yet implemented on Windows");
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("dunstctl").arg("set-paused").arg("toggle").output();
    }
}

/// Take a screenshot
pub fn take_screenshot() {
    log::info!("[SCREENSHOT] Taking screenshot");

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("screencapture")
            .args(["-ic"])
            .output();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("snippingtool").output();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("gnome-screenshot")
            .args(["-a", "-c"])
            .output()
            .or_else(|_| Command::new("scrot").args(["-s", "-"]).output());
    }
}

/// Lock the screen
pub fn lock_screen() {
    log::info!("[LOCK] Locking screen");

    #[cfg(target_os = "macos")]
    {
        let script = r#"tell application "System Events" to keystroke "q" using {command down, control down}"#;
        let _ = Command::new("osascript").args(["-e", script]).output();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("rundll32.exe")
            .args(["user32.dll,LockWorkStation"])
            .output();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("loginctl").arg("lock-session").output();
    }
}

/// Send a keyboard shortcut (Cmd+key on macOS, Ctrl+key on Windows/Linux)
pub fn send_keyboard_shortcut(key: &str) {
    log::info!("[SHORTCUT] Sending keyboard shortcut: {}", key);

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to keystroke "{}" using command down"#,
            key
        );
        let _ = Command::new("osascript").args(["-e", &script]).output();
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VK_CONTROL,
        };

        let vk_key = match key {
            "b" => 0x42u16,
            "i" => 0x49u16,
            "u" => 0x55u16,
            _ => return,
        };

        let inputs: [INPUT; 4] = [
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_key), wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_key), wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 } } },
            INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 } } },
        ];
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    }

    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        if wayland {
            let _ = Command::new("wtype")
                .args(["-M", "ctrl", key, "-m", "ctrl"])
                .output();
        } else {
            let _ = Command::new("xdotool")
                .args(["key", "--clearmodifiers", &format!("ctrl+{}", key)])
                .output();
        }
    }
}

/// Create a new note in Apple Notes (macOS only)
#[cfg(target_os = "macos")]
pub fn apple_notes_create(title: &str, body: &str) -> Result<(), String> {
    log::info!("[NOTES] Creating Apple Note: {}", title);
    let escaped_title = title.replace("\\", "\\\\").replace("\"", "\\\"");
    let escaped_body = body.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n");
    let script = format!(
        r#"tell application "Notes"
            activate
            tell account "iCloud"
                make new note at folder "Notes" with properties {{name:"{}", body:"{}"}}
            end tell
        end tell"#,
        escaped_title, escaped_body
    );
    match Command::new("osascript").args(["-e", &script]).output() {
        Ok(output) => {
            if output.status.success() {
                log::info!("[NOTES] Apple Note created successfully");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("AppleScript error: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to execute osascript: {}", e)),
    }
}

/// Create a markdown note in an Obsidian vault
pub fn obsidian_create(vault_path: &str, title: &str, body: &str) -> Result<(), String> {
    log::info!("[OBSIDIAN] Creating note in vault: {}", vault_path);
    let vault = std::path::Path::new(vault_path);
    if !vault.exists() {
        return Err(format!("Vault path does not exist: {}", vault_path));
    }

    // Sanitize title for filename
    let safe_title: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let file_path = vault.join(format!("{}.md", safe_title.trim()));

    let content = format!("# {}\n\n{}\n\n---\n*Cree par Phonon le {}*\n",
        title, body, chrono::Local::now().format("%Y-%m-%d %H:%M"));

    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write note: {}", e))?;

    log::info!("[OBSIDIAN] Note created at: {:?}", file_path);
    Ok(())
}
