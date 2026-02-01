//! Keybind configuration: built-in defaults and optional user override from
//! `$XDG_CONFIG_HOME/vibevim/keybinds.json` (or `~/.config/vibevim/keybinds.json`).

use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A single key (code + modifiers) that can be matched against a KeyEvent.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParsedKey {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl ParsedKey {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        key.code == self.code && key.modifiers == self.modifiers
    }

    /// Match key, allowing Shift for the second key of a chord (e.g. "e" matches both e and E).
    pub fn matches_allow_shift(&self, key: &KeyEvent) -> bool {
        if key.code != self.code {
            return false;
        }
        let forbidden = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER;
        !key.modifiers.intersects(forbidden) && !self.modifiers.intersects(forbidden)
    }
}

/// A binding is either a single key or a two-key chord.
#[derive(Clone, Debug)]
pub enum Binding {
    Single(ParsedKey),
    Chord(ParsedKey, ParsedKey),
}

impl Binding {
    #[allow(dead_code)]
    pub fn first_key(&self) -> &ParsedKey {
        match self {
            Binding::Single(p) => p,
            Binding::Chord(p, _) => p,
        }
    }

    #[allow(dead_code)]
    pub fn second_key(&self) -> Option<&ParsedKey> {
        match self {
            Binding::Single(_) => None,
            Binding::Chord(_, p) => Some(p),
        }
    }

    pub fn matches_first_key(&self, key: &KeyEvent) -> bool {
        match self {
            Binding::Single(p) => p.matches(key),
            Binding::Chord(p, _) => p.matches(key),
        }
    }

    pub fn matches_second_key(&self, key: &KeyEvent) -> bool {
        match self {
            Binding::Single(_) => false,
            Binding::Chord(_, second) => second.matches_allow_shift(key),
        }
    }

    pub fn is_chord(&self) -> bool {
        matches!(self, Binding::Chord(_, _))
    }
}

/// Per-context map: action name -> list of bindings.
pub type ContextKeybinds = HashMap<String, Vec<Binding>>;

/// Full keybind map: context name -> context keybinds.
pub type KeybindMap = HashMap<String, ContextKeybinds>;

/// Parse a key string into ParsedKey. Examples: "h", "Left", "Ctrl+c", "F5", "Space".
pub fn parse_key(s: &str) -> Option<ParsedKey> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    let (modifiers, key_part) = if parts.len() >= 2 {
        let mut mods = KeyModifiers::empty();
        for p in &parts[..parts.len() - 1] {
            match *p {
                "Ctrl" | "Control" => mods.insert(KeyModifiers::CONTROL),
                "Shift" => mods.insert(KeyModifiers::SHIFT),
                "Alt" => mods.insert(KeyModifiers::ALT),
                "Super" | "Meta" => mods.insert(KeyModifiers::SUPER),
                _ => return None,
            }
        }
        (mods, parts[parts.len() - 1])
    } else {
        (KeyModifiers::empty(), parts[0])
    };

    let code = match key_part {
        "Enter" | "Return" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Tab" => KeyCode::Tab,
        "Esc" | "Escape" => KeyCode::Esc,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Space" => KeyCode::Char(' '),
        "F1" => KeyCode::F(1),
        "F2" => KeyCode::F(2),
        "F3" => KeyCode::F(3),
        "F4" => KeyCode::F(4),
        "F5" => KeyCode::F(5),
        "F6" => KeyCode::F(6),
        "F7" => KeyCode::F(7),
        "F8" => KeyCode::F(8),
        "F9" => KeyCode::F(9),
        "F10" => KeyCode::F(10),
        "F11" => KeyCode::F(11),
        "F12" => KeyCode::F(12),
        _ if key_part.len() == 1 => {
            let c = key_part.chars().next().unwrap();
            KeyCode::Char(c)
        }
        _ if key_part.len() == 2 && key_part.starts_with('F') => {
            if let Ok(n) = key_part[1..].parse::<u8>() {
                KeyCode::F(n)
            } else {
                return None;
            }
        }
        _ => return None,
    };

    Some(ParsedKey { code, modifiers })
}

/// Parse a binding string: "h" or "Space e" or "Ctrl+w w".
pub fn parse_binding(s: &str) -> Option<Binding> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 1 {
        parse_key(parts[0]).map(Binding::Single)
    } else if parts.len() == 2 {
        let first = parse_key(parts[0])?;
        let second = parse_key(parts[1])?;
        Some(Binding::Chord(first, second))
    } else {
        None
    }
}

/// Config directory for vibevim: $XDG_CONFIG_HOME/vibevim or $HOME/.config/vibevim.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("vibevim"))
}

/// Path to keybinds.json.
pub fn keybinds_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("keybinds.json"))
}

/// Load user keybinds from keybinds.json. Returns None if file missing or invalid.
pub fn load_user_keybinds() -> Option<KeybindMap> {
    let path = keybinds_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    let raw: HashMap<String, HashMap<String, Vec<String>>> = serde_json::from_str(&contents).ok()?;
    let mut result = KeybindMap::new();
    for (context, actions) in raw {
        let mut ctx_binds = ContextKeybinds::new();
        for (action, key_strs) in actions {
            let bindings: Vec<Binding> = key_strs
                .iter()
                .filter_map(|s| parse_binding(s))
                .collect();
            if !bindings.is_empty() {
                ctx_binds.insert(action, bindings);
            }
        }
        result.insert(context, ctx_binds);
    }
    Some(result)
}

/// Merge user keybinds on top of defaults. User entries replace default bindings for that action.
pub fn merge_keybinds(default: KeybindMap, user: KeybindMap) -> KeybindMap {
    let mut out = default;
    for (context, user_actions) in user {
        let ctx = out.entry(context).or_default();
        for (action, bindings) in user_actions {
            ctx.insert(action, bindings);
        }
    }
    out
}

/// Build the default keybind map (current hardcoded behavior).
pub fn default_keybinds() -> KeybindMap {
    let mut m = KeybindMap::new();

    // Global
    let mut global = ContextKeybinds::new();
    global.insert(
        "toggle_sidebar".to_string(),
        vec![
            parse_binding("Space e").unwrap(),
            parse_binding("Space E").unwrap(),
        ],
    );
    global.insert(
        "focus_explorer_toggle".to_string(),
        vec![parse_binding("Ctrl+w w").unwrap()],
    );
    global.insert(
        "enter_command_mode".to_string(),
        vec![parse_binding(":").unwrap()],
    );
    m.insert("global".to_string(), global);

    // Explorer
    let mut explorer = ContextKeybinds::new();
    explorer.insert(
        "refresh".to_string(),
        vec![
            parse_binding("r").unwrap(),
            parse_binding("R").unwrap(),
            parse_binding("F5").unwrap(),
        ],
    );
    explorer.insert(
        "open_enter".to_string(),
        vec![
            parse_binding("Enter").unwrap(),
            parse_binding("l").unwrap(),
            parse_binding("Right").unwrap(),
        ],
    );
    m.insert("explorer".to_string(), explorer);

    // Normal
    let mut normal = ContextKeybinds::new();
    normal.insert("move_left".to_string(), vec![parse_binding("h").unwrap(), parse_binding("Left").unwrap()]);
    normal.insert("move_down".to_string(), vec![parse_binding("j").unwrap(), parse_binding("Down").unwrap()]);
    normal.insert("move_up".to_string(), vec![parse_binding("k").unwrap(), parse_binding("Up").unwrap()]);
    normal.insert("move_right".to_string(), vec![parse_binding("l").unwrap(), parse_binding("Right").unwrap()]);
    normal.insert("move_word_forward".to_string(), vec![parse_binding("w").unwrap()]);
    normal.insert("move_word_backward".to_string(), vec![parse_binding("b").unwrap()]);
    normal.insert("move_to_end_of_word".to_string(), vec![parse_binding("e").unwrap()]);
    normal.insert("move_word_forward_W".to_string(), vec![parse_binding("W").unwrap()]);
    normal.insert("move_word_backward_B".to_string(), vec![parse_binding("B").unwrap()]);
    normal.insert("move_to_end_of_word_E".to_string(), vec![parse_binding("E").unwrap()]);
    normal.insert("move_to_line_start".to_string(), vec![parse_binding("0").unwrap()]);
    normal.insert("move_to_line_end".to_string(), vec![parse_binding("$").unwrap()]);
    normal.insert("move_to_first_non_blank".to_string(), vec![parse_binding("^").unwrap()]);
    normal.insert("move_to_last_line".to_string(), vec![parse_binding("G").unwrap()]);
    normal.insert("move_paragraph_prev".to_string(), vec![parse_binding("{").unwrap()]);
    normal.insert("move_paragraph_next".to_string(), vec![parse_binding("}").unwrap()]);
    normal.insert("move_to_first_line".to_string(), vec![parse_binding("g g").unwrap()]);
    normal.insert("enter_insert_mode".to_string(), vec![parse_binding("i").unwrap()]);
    normal.insert("enter_insert_mode_append".to_string(), vec![parse_binding("a").unwrap()]);
    normal.insert("enter_insert_mode_end".to_string(), vec![parse_binding("A").unwrap()]);
    normal.insert("enter_insert_mode_start".to_string(), vec![parse_binding("I").unwrap()]);
    normal.insert("open_line_below".to_string(), vec![parse_binding("o").unwrap()]);
    normal.insert("open_line_above".to_string(), vec![parse_binding("O").unwrap()]);
    normal.insert("delete_char_at_cursor".to_string(), vec![parse_binding("x").unwrap()]);
    normal.insert("delete_to_end_of_line".to_string(), vec![parse_binding("D").unwrap()]);
    normal.insert("join_lines".to_string(), vec![parse_binding("J").unwrap()]);
    normal.insert("delete_current_line".to_string(), vec![parse_binding("d d").unwrap()]);
    normal.insert("replace_char".to_string(), vec![parse_binding("r").unwrap()]);
    normal.insert("enter_command_mode".to_string(), vec![parse_binding(":").unwrap()]);
    normal.insert("enter_search_mode".to_string(), vec![parse_binding("/").unwrap()]);
    normal.insert("repeat_search_forward".to_string(), vec![parse_binding("n").unwrap()]);
    normal.insert("repeat_search_backward".to_string(), vec![parse_binding("N").unwrap()]);
    normal.insert("return_to_normal".to_string(), vec![parse_binding("Ctrl+c").unwrap()]);
    m.insert("normal".to_string(), normal);

    // Insert
    let mut insert = ContextKeybinds::new();
    insert.insert("enter_normal_mode".to_string(), vec![parse_binding("Esc").unwrap()]);
    insert.insert("backspace".to_string(), vec![parse_binding("Backspace").unwrap()]);
    insert.insert("insert_newline".to_string(), vec![parse_binding("Enter").unwrap()]);
    insert.insert("return_to_normal".to_string(), vec![parse_binding("Ctrl+c").unwrap()]);
    insert.insert("move_left".to_string(), vec![parse_binding("Left").unwrap()]);
    insert.insert("move_right".to_string(), vec![parse_binding("Right").unwrap()]);
    insert.insert("move_up".to_string(), vec![parse_binding("Up").unwrap()]);
    insert.insert("move_down".to_string(), vec![parse_binding("Down").unwrap()]);
    insert.insert("insert_tab".to_string(), vec![parse_binding("Tab").unwrap()]);
    m.insert("insert".to_string(), insert);

    // Command
    let mut command = ContextKeybinds::new();
    command.insert("cancel".to_string(), vec![parse_binding("Esc").unwrap()]);
    command.insert("execute".to_string(), vec![parse_binding("Enter").unwrap()]);
    command.insert("backspace".to_string(), vec![parse_binding("Backspace").unwrap()]);
    m.insert("command".to_string(), command);

    // Search
    let mut search = ContextKeybinds::new();
    search.insert("cancel".to_string(), vec![parse_binding("Esc").unwrap()]);
    search.insert("search_forward".to_string(), vec![parse_binding("Enter").unwrap()]);
    search.insert("backspace".to_string(), vec![parse_binding("Backspace").unwrap()]);
    m.insert("search".to_string(), search);

    m
}

/// Resolve current key (and optional pending second key) to an action name in the given context.
/// Returns (action_name, is_second_key_of_chord). If pending_action/second_key are given and key
/// matches second key of that binding, returns (action, true). Else looks for single-key or
/// first-key-of-chord match.
pub fn resolve_action(
    keybinds: &KeybindMap,
    context: &str,
    key: &KeyEvent,
    pending_chord: Option<(&str, &ParsedKey)>,
) -> Option<(String, bool)> {
    let ctx = keybinds.get(context)?;
    if let Some((action, _second)) = pending_chord {
        if let Some(bindings) = ctx.get(action) {
            for b in bindings {
                if b.matches_second_key(key) {
                    return Some((action.to_string(), true));
                }
            }
        }
    }
    for (action, bindings) in ctx {
        for b in bindings {
            if b.matches_first_key(key) {
                return Some((action.clone(), b.is_chord()));
            }
        }
    }
    None
}

/// When key matches the first key of a chord, return (action_name, second_key) so caller can set pending.
pub fn resolve_first_key_chord(
    keybinds: &KeybindMap,
    context: &str,
    key: &KeyEvent,
) -> Option<(String, ParsedKey)> {
    let ctx = keybinds.get(context)?;
    for (action, bindings) in ctx {
        for b in bindings {
            if let Binding::Chord(first, second) = b {
                if first.matches(key) {
                    return Some((action.clone(), second.clone()));
                }
            }
        }
    }
    None
}

/// Find which chord binding (action, second_key) is waiting for this key. Used when we're in
/// "pending first key" state and need to know which action's second key we're matching.
#[allow(dead_code)]
pub fn resolve_chord_second(
    keybinds: &KeybindMap,
    context: &str,
    key: &KeyEvent,
    pending_first_key: &ParsedKey,
) -> Option<String> {
    let ctx = keybinds.get(context)?;
    for (action, bindings) in ctx {
        for b in bindings {
            if let Binding::Chord(first, second) = b {
                if first == pending_first_key && second.matches_allow_shift(key) {
                    return Some(action.clone());
                }
            }
        }
    }
    None
}
