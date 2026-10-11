//! 全局快捷键：把用户写的加速键字符串解析成 `Shortcut`，并维护注册状态。
//!
//! 解析是自己实现的（不依赖 `Shortcut::from_str`），好处是：
//! - 别名可控（`CmdOrCtrl` / `Super` / `Win` 等在 Windows 上一律映射为 Ctrl）；
//! - 拒绝“只有 Shift”这种不安全的组合；
//! - 报错信息是明确的中文，能直接弹给用户看。

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

/// 错误：一个组合键里出现了两个按键（例：`Ctrl+A+B`）。
pub const ERR_MULTIPLE_KEYS: &str = "只能包含一个按键";
/// 错误：缺少非 Shift 的修饰键。
pub const ERR_NO_MODIFIER: &str = "需要至少一个修饰键（Ctrl / Alt / Shift）";
/// 错误：空字符串。
pub const ERR_EMPTY: &str = "快捷键不能为空";
/// 错误：结束在 `+` 上。
pub const ERR_DANGLING: &str = "快捷键不能以 + 结尾";

/// 把加速键字符串解析为具体的 `Shortcut`。
///
/// 支持 `Ctrl+Alt+Space` 这类写法，大小写不敏感。修饰键别名：
/// `CmdOrCtrl` / `CommandOrControl` / `CommandOrCtrl` / `Command` / `Cmd` /
/// `Super` / `Meta` / `Win` 在 Windows 上等同 `Ctrl`。
pub fn parse_accelerator(accelerator: &str) -> Result<Shortcut, String> {
    let input = accelerator.trim();
    if input.is_empty() {
        return Err(ERR_EMPTY.to_string());
    }

    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;

    for raw in input.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            return Err(ERR_DANGLING.to_string());
        }
        if let Some(flag) = parse_modifier(token) {
            mods |= flag;
            continue;
        }
        let code = parse_key(token).ok_or_else(|| format!("无法识别的按键：{token}"))?;
        if key.replace(code).is_some() {
            return Err(ERR_MULTIPLE_KEYS.to_string());
        }
    }

    let key = key.ok_or_else(|| ERR_NO_MODIFIER.to_string())?;
    // Shift 单独存在（Shift+A）容易和系统/输入法冲突，也容易误触，直接拒绝。
    if (mods & !Modifiers::SHIFT).is_empty() {
        return Err(ERR_NO_MODIFIER.to_string());
    }
    Ok(Shortcut::new(Some(mods), key))
}

/// 修饰键别名解析；不是修饰键则返回 `None`。
fn parse_modifier(token: &str) -> Option<Modifiers> {
    match token.to_ascii_uppercase().as_str() {
        // Windows 上没有 Command，按约定统一落到 Ctrl。
        "CMDORCTRL" | "COMMANDORCONTROL" | "COMMANDORCTRL" | "COMMAND" | "CMD" | "SUPER"
        | "META" | "WIN" | "CONTROL" | "CTRL" => Some(Modifiers::CONTROL),
        "ALT" | "OPTION" => Some(Modifiers::ALT),
        "SHIFT" => Some(Modifiers::SHIFT),
        _ => None,
    }
}

/// 按键名 → 物理键位码。名字与 US 布局一致，大小写不敏感。
fn parse_key(token: &str) -> Option<Code> {
    let upper = token.to_ascii_uppercase();
    match upper.as_str() {
        "SPACE" | "SPACEBAR" => return Some(Code::Space),
        "ENTER" | "RETURN" => return Some(Code::Enter),
        "TAB" => return Some(Code::Tab),
        "BACKSPACE" => return Some(Code::Backspace),
        "ESCAPE" | "ESC" => return Some(Code::Escape),
        "DELETE" | "DEL" => return Some(Code::Delete),
        "INSERT" | "INS" => return Some(Code::Insert),
        "HOME" => return Some(Code::Home),
        "END" => return Some(Code::End),
        "PAGEUP" | "PGUP" => return Some(Code::PageUp),
        "PAGEDOWN" | "PGDN" => return Some(Code::PageDown),
        "UP" => return Some(Code::ArrowUp),
        "DOWN" => return Some(Code::ArrowDown),
        "LEFT" => return Some(Code::ArrowLeft),
        "RIGHT" => return Some(Code::ArrowRight),
        // 标点键的“单词写法”，方便前端做录制控件时直接用 Code 名。
        "BACKQUOTE" | "BACKTICK" => return Some(Code::Backquote),
        "MINUS" => return Some(Code::Minus),
        "EQUAL" | "EQUALS" | "PLUS" => return Some(Code::Equal),
        "BRACKETLEFT" | "LEFTBRACKET" => return Some(Code::BracketLeft),
        "BRACKETRIGHT" | "RIGHTBRACKET" => return Some(Code::BracketRight),
        "BACKSLASH" => return Some(Code::Backslash),
        "SEMICOLON" => return Some(Code::Semicolon),
        "QUOTE" | "APOSTROPHE" => return Some(Code::Quote),
        "COMMA" => return Some(Code::Comma),
        "PERIOD" | "DOT" => return Some(Code::Period),
        "SLASH" => return Some(Code::Slash),
        _ => {}
    }

    // F1..=F24
    if let Some(digits) = upper.strip_prefix('F')
        && let Ok(n) = digits.parse::<u32>()
        && (1..=24).contains(&n)
    {
        return function_key(n);
    }

    // 单个字符
    let mut chars = token.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if ch.is_ascii_alphabetic() {
        return letter_code(ch.to_ascii_uppercase());
    }
    if ch.is_ascii_digit() {
        return digit_code(ch);
    }
    punctuation_code(ch)
}

fn function_key(n: u32) -> Option<Code> {
    Some(match n {
        1 => Code::F1,
        2 => Code::F2,
        3 => Code::F3,
        4 => Code::F4,
        5 => Code::F5,
        6 => Code::F6,
        7 => Code::F7,
        8 => Code::F8,
        9 => Code::F9,
        10 => Code::F10,
        11 => Code::F11,
        12 => Code::F12,
        13 => Code::F13,
        14 => Code::F14,
        15 => Code::F15,
        16 => Code::F16,
        17 => Code::F17,
        18 => Code::F18,
        19 => Code::F19,
        20 => Code::F20,
        21 => Code::F21,
        22 => Code::F22,
        23 => Code::F23,
        24 => Code::F24,
        _ => return None,
    })
}

fn letter_code(upper: char) -> Option<Code> {
    Some(match upper {
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        'Z' => Code::KeyZ,
        _ => return None,
    })
}

fn digit_code(digit: char) -> Option<Code> {
    Some(match digit {
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        '9' => Code::Digit9,
        _ => return None,
    })
}

fn punctuation_code(ch: char) -> Option<Code> {
    Some(match ch {
        '`' => Code::Backquote,
        '-' => Code::Minus,
        '=' => Code::Equal,
        '[' => Code::BracketLeft,
        ']' => Code::BracketRight,
        '\\' => Code::Backslash,
        ';' => Code::Semicolon,
        '\'' => Code::Quote,
        ',' => Code::Comma,
        '.' => Code::Period,
        '/' => Code::Slash,
        _ => return None,
    })
}

/// 注册初始快捷键（应用启动时调用一次）。
pub fn register_initial<R: Runtime>(
    app: &AppHandle<R>,
    accelerator: &str,
) -> Result<Shortcut, String> {
    let shortcut = parse_accelerator(accelerator)?;
    register_shortcut(app, shortcut)?;
    Ok(shortcut)
}

/// 换绑快捷键：先解析 → 注销旧的 → 注册新的；失败则把旧的重新注册回来。
///
/// 参数 `current` 是当前已注册的组合（首次注册失败时为 `None`）。
/// 不负责持久化——持久化由前端调用 `save_data` 完成。
pub fn rebind<R: Runtime>(
    app: &AppHandle<R>,
    current: Option<Shortcut>,
    accelerator: &str,
) -> Result<Shortcut, String> {
    let next = parse_accelerator(accelerator)?;
    if Some(next) == current {
        // 没变化，直接成功，避免无谓的注销/注册（也会让系统短暂丢失热键）。
        return Ok(next);
    }

    if let Some(prev) = current {
        // 旧的先注销，否则系统会认为新组合“已被占用”。
        if let Err(e) = app.global_shortcut().unregister(prev) {
            eprintln!("注销旧快捷键失败：{e}");
        }
    }

    match app.global_shortcut().register(next) {
        Ok(()) => Ok(next),
        Err(e) => {
            // 回滚：保留原来的快捷键仍然可用。
            if let Some(prev) = current
                && let Err(rollback) = app.global_shortcut().register(prev)
            {
                eprintln!("回滚旧快捷键失败：{rollback}");
            }
            Err(format!("快捷键注册失败（可能已被其它程序占用）：{e}"))
        }
    }
}

fn register_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: Shortcut) -> Result<(), String> {
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("快捷键注册失败（可能已被其它程序占用）：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(s: &str) -> Shortcut {
        parse_accelerator(s).expect("应当解析成功")
    }

    #[test]
    fn parses_default_hotkey() {
        let s = acc("Ctrl+Alt+Space");
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(s.key, Code::Space);
    }

    #[test]
    fn parses_letters_digits_and_function_keys() {
        assert_eq!(acc("Ctrl+Shift+A").key, Code::KeyA);
        assert_eq!(acc("Ctrl+Shift+a").key, Code::KeyA);
        assert_eq!(acc("Alt+1").key, Code::Digit1);
        assert_eq!(acc("Alt+F12").key, Code::F12);
        assert_eq!(acc("Ctrl+F24").key, Code::F24);
        assert_eq!(acc("Ctrl+Enter").key, Code::Enter);
        assert_eq!(acc("Ctrl+Return").key, Code::Enter);
        assert_eq!(acc("Ctrl+Esc").key, Code::Escape);
        assert_eq!(acc("Ctrl+Escape").key, Code::Escape);
        assert_eq!(acc("Ctrl+Up").key, Code::ArrowUp);
        assert_eq!(acc("Ctrl+PageDown").key, Code::PageDown);
        assert_eq!(acc("Ctrl+Backquote").key, Code::Backquote);
        assert_eq!(acc("Ctrl+`").key, Code::Backquote);
        assert_eq!(acc("Ctrl+`").mods, Modifiers::CONTROL);
        assert_eq!(acc("Ctrl+Backtick").key, Code::Backquote);
        assert_eq!(acc("Ctrl+LeftBracket").key, Code::BracketLeft);
        assert_eq!(acc("Ctrl+[").key, Code::BracketLeft);
        assert_eq!(acc("Ctrl+Slash").key, Code::Slash);
        assert_eq!(acc("Ctrl+/").key, Code::Slash);
        assert_eq!(acc("Ctrl+Apostrophe").key, Code::Quote);
        assert_eq!(acc("Ctrl+'").key, Code::Quote);
        assert_eq!(acc("Ctrl+Equals").key, Code::Equal);
        assert_eq!(acc("Ctrl+=").key, Code::Equal);
        assert_eq!(acc("Ctrl+Semicolon").key, Code::Semicolon);
        assert_eq!(acc("Ctrl+;").key, Code::Semicolon);
        assert_eq!(acc("Ctrl+Comma").key, Code::Comma);
        assert_eq!(acc("Ctrl+,").key, Code::Comma);
        assert_eq!(acc("Ctrl+Period").key, Code::Period);
        assert_eq!(acc("Ctrl+.").key, Code::Period);
        assert_eq!(acc("Ctrl+Backslash").key, Code::Backslash);
        assert_eq!(acc("Ctrl+\\").key, Code::Backslash);
        assert_eq!(acc("Ctrl+Minus").key, Code::Minus);
        assert_eq!(acc("Ctrl+-").key, Code::Minus);
    }

    #[test]
    fn normalises_modifier_aliases_to_control_on_windows() {
        for alias in [
            "CmdOrCtrl",
            "CommandOrControl",
            "CommandOrCtrl",
            "Command",
            "Cmd",
            "Super",
            "Meta",
            "Win",
            "Control",
            "Ctrl",
            "cTrL",
        ] {
            let s = acc(&format!("{alias}+K"));
            assert_eq!(s.mods, Modifiers::CONTROL, "别名 {alias} 应映射为 Ctrl");
            assert_eq!(s.key, Code::KeyK);
        }
        assert_eq!(acc("Option+K").mods, Modifiers::ALT);
        assert_eq!(acc("Alt+Shift+K").mods, Modifiers::ALT | Modifiers::SHIFT);
    }

    #[test]
    fn rejects_missing_non_shift_modifier() {
        assert_eq!(parse_accelerator("A"), Err(ERR_NO_MODIFIER.to_string()));
        assert_eq!(parse_accelerator("F5"), Err(ERR_NO_MODIFIER.to_string()));
        assert_eq!(
            parse_accelerator("Shift+A"),
            Err(ERR_NO_MODIFIER.to_string())
        );
        assert_eq!(parse_accelerator("Shift"), Err(ERR_NO_MODIFIER.to_string()));
        assert_eq!(parse_accelerator("Ctrl"), Err(ERR_NO_MODIFIER.to_string()));
        assert_eq!(parse_accelerator(""), Err(ERR_EMPTY.to_string()));
        assert_eq!(parse_accelerator("   "), Err(ERR_EMPTY.to_string()));
    }

    #[test]
    fn rejects_unknown_key() {
        assert_eq!(
            parse_accelerator("Ctrl+Foo"),
            Err("无法识别的按键：Foo".to_string())
        );
        assert_eq!(
            parse_accelerator("Ctrl+F25"),
            Err("无法识别的按键：F25".to_string())
        );
        assert_eq!(
            parse_accelerator("Ctrl+CtrlX"),
            Err("无法识别的按键：CtrlX".to_string())
        );
    }

    #[test]
    fn rejects_two_keys_and_dangling_plus() {
        assert_eq!(
            parse_accelerator("Ctrl+A+B"),
            Err(ERR_MULTIPLE_KEYS.to_string())
        );
        assert_eq!(parse_accelerator("Ctrl+A+"), Err(ERR_DANGLING.to_string()));
        assert_eq!(parse_accelerator("Ctrl++A"), Err(ERR_DANGLING.to_string()));
        assert_eq!(parse_accelerator("+"), Err(ERR_DANGLING.to_string()));
    }

    #[test]
    fn tolerates_whitespace_and_case() {
        let s = acc("  ctrl + alt + space ");
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(s.key, Code::Space);
    }

    #[test]
    fn same_text_parses_to_equal_shortcuts() {
        // `rebind` 的“无变化就跳过”依赖这一点：等价写法必须解析出相等的 Shortcut。
        assert_eq!(acc("Ctrl+Alt+Space"), acc("ctrl + alt + space"));
        assert_eq!(acc("CmdOrCtrl+Space"), acc("Ctrl+Space"));
        assert_eq!(acc("Ctrl+Return"), acc("Ctrl+Enter"));
        assert_ne!(acc("Ctrl+Space"), acc("Alt+Space"));
        assert_ne!(acc("Ctrl+Space"), acc("Ctrl+Shift+Space"));
        assert_ne!(acc("Ctrl+A"), acc("Ctrl+B"));
    }
}
