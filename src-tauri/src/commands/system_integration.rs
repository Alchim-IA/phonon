use std::process::Command;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Copies text to the system clipboard
fn copy_to_clipboard(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))
}

/// Simulates Cmd+V paste keystroke via AppleScript (macOS)
#[cfg(target_os = "macos")]
fn simulate_paste() -> Result<(), String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to keystroke \"v\" using command down",
        ])
        .output()
        .map_err(|e| format!("Failed to execute AppleScript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("AppleScript failed: {}", stderr));
    }

    Ok(())
}

/// Simulates Ctrl+V paste keystroke via SendInput API (Windows)
#[cfg(target_os = "windows")]
fn simulate_paste() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };

    // Create input events for Ctrl+V
    // 1. Press Ctrl
    // 2. Press V
    // 3. Release V
    // 4. Release Ctrl
    let inputs: [INPUT; 4] = [
        // Ctrl down
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // V down
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // V up
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // Ctrl up
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };

    if sent != 4 {
        return Err(format!("SendInput failed: only {} of 4 inputs sent", sent));
    }

    Ok(())
}

/// Simulates Ctrl+V paste keystroke on Linux (X11 via xdotool, Wayland via wtype/ydotool)
#[cfg(target_os = "linux")]
fn simulate_paste() -> Result<(), String> {
    // Detect display server
    let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

    if wayland {
        // Try wtype first (more common on Wayland)
        let wtype_result = Command::new("wtype")
            .args(["-M", "ctrl", "v", "-m", "ctrl"])
            .output();

        match wtype_result {
            Ok(output) if output.status.success() => return Ok(()),
            _ => {
                // Fallback to ydotool (requires ydotoold daemon)
                let ydotool_result = Command::new("ydotool")
                    .args(["key", "29:1", "47:1", "47:0", "29:0"]) // Ctrl down, V down, V up, Ctrl up
                    .output();

                match ydotool_result {
                    Ok(output) if output.status.success() => return Ok(()),
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(format!(
                            "Wayland paste failed. Install 'wtype' or 'ydotool'. Error: {}",
                            stderr
                        ));
                    }
                    Err(e) => {
                        return Err(format!(
                            "Wayland paste failed. Install 'wtype' or 'ydotool'. Error: {}",
                            e
                        ));
                    }
                }
            }
        }
    } else {
        // X11 - use xdotool
        let output = Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .output()
            .map_err(|e| format!("Failed to execute xdotool. Install it with 'sudo apt install xdotool'. Error: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("xdotool failed: {}", stderr));
        }

        Ok(())
    }
}

/// Fallback for other platforms (BSD, etc.)
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn simulate_paste() -> Result<(), String> {
    Err("Auto-paste not supported on this platform".to_string())
}

/// Pastes text into the active application by copying to clipboard and simulating paste keystroke
///
/// # Arguments
/// * `app` - Tauri app handle for clipboard access
/// * `text` - Text to paste into the active application
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error description on failure
#[tauri::command]
pub async fn auto_paste(app: AppHandle, text: String) -> Result<(), String> {
    // 1. Copy text to clipboard
    copy_to_clipboard(&app, &text)?;

    // 2. Small delay to ensure clipboard is updated and focus is stable
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 3. Simulate paste keystroke
    simulate_paste()?;

    Ok(())
}

// ============================================================================
// Floating Window Management
// ============================================================================

/// Gets the floating window handle
fn get_floating_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("floating")
        .ok_or_else(|| "Floating window not found".to_string())
}

/// Shows the floating window
#[tauri::command]
pub fn show_floating_window(app: AppHandle) -> Result<(), String> {
    let window = get_floating_window(&app)?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Hides the floating window
#[tauri::command]
pub fn hide_floating_window(app: AppHandle) -> Result<(), String> {
    let window = get_floating_window(&app)?;
    window.hide().map_err(|e| e.to_string())
}

/// Toggles the floating window visibility
#[tauri::command]
pub fn toggle_floating_window(app: AppHandle) -> Result<bool, String> {
    let window = get_floating_window(&app)?;
    let is_visible = window.is_visible().map_err(|e| e.to_string())?;

    if is_visible {
        window.hide().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// Sets the floating window size (for compact/extended states)
#[tauri::command]
pub fn set_floating_window_size(app: AppHandle, width: u32, height: u32) -> Result<(), String> {
    let window = get_floating_window(&app)?;
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|e| e.to_string())
}

/// Gets the floating window position
#[tauri::command]
pub fn get_floating_window_position(app: AppHandle) -> Result<Option<(i32, i32)>, String> {
    let window = get_floating_window(&app)?;
    match window.outer_position() {
        Ok(pos) => Ok(Some((pos.x, pos.y))),
        Err(_) => Ok(None),
    }
}

/// Sets the floating window position
#[tauri::command]
pub fn set_floating_window_position(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    let window = get_floating_window(&app)?;
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())
}
