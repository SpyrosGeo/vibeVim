# VibeVim 
  A modal terminal text editor written entirely by vibe coding in Rust, with vim-like keybindings.
  ## Features
  - **Modes**: Normal, Insert, and Command (`:`) mode
  - **Motion**: h/j/k/l, w/b/e, 0/$/^, gg/G, {/}, W/B/E
  - **Insert**: i, a, A, I, o, O (open line below/above)
  - **Edit**: x (delete char), D (delete to EOL), dd (delete line), J (join lines), r (replace char)
  - **Commands**: :w, :wq, :q, :q!, :w &lt;filename&gt;
  - **Misc**: Ctrl+C returns to normal mode (does not quit)
  - You want to quit use the command mode like a man
  ## Build

  cargo build          # debug
  cargo build --release  # release

  ## Run

  cargo run                  # new buffer
  cargo run path/to/file     # open or create file
  ./target/release/terminal-editor path/to/file

  ## Requirements
  - Rust (edition 2021)
  - Crossterm, ratatui, ropey (see Cargo.toml)
