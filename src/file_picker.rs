//! Fuzzy file picker: Space p to open, nucleo for matching, walkdir for file list.

use std::path::PathBuf;
use std::sync::Arc;

use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Injector, Nucleo, Utf32String};
use walkdir::WalkDir;

const MAX_FILES: usize = 10_000;

/// State for the fuzzy file picker modal (Space p).
pub struct FilePickerState {
    /// Directory we listed files from
    #[allow(dead_code)]
    pub search_root: PathBuf,
    /// Current search query (user input)
    pub query: String,
    /// Nucleo fuzzy matcher; item type = full path string
    pub nucleo: Nucleo<String>,
    /// Injector handle (kept so items stay in nucleo)
    pub _injector: Injector<String>,
    /// Selected index in the matched list (0..matched_item_count)
    pub selected_index: usize,
    /// Scroll offset for list view (so selection stays visible)
    #[allow(dead_code)]
    pub scroll_offset: usize,
}

impl FilePickerState {
    /// Build list of file paths under `root` (files only, not dirs). Returns error if walk fails.
    fn collect_paths(root: &std::path::Path) -> Result<Vec<String>, std::io::Error> {
        let mut paths = Vec::new();
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(s) = path.to_str() {
                    paths.push(s.to_string());
                    if paths.len() >= MAX_FILES {
                        break;
                    }
                }
            }
        }
        Ok(paths)
    }

    /// Create a new file picker state: walk directory, create Nucleo, inject all paths.
    pub fn new(search_root: PathBuf) -> Result<Self, String> {
        let paths = Self::collect_paths(&search_root).map_err(|e| e.to_string())?;

        let config = Config::DEFAULT.match_paths();
        let notify = Arc::new(|| {});
        let mut nucleo = Nucleo::new(config, notify, None, 1);
        let injector = nucleo.injector();

        for path in &paths {
            injector.push(
                path.clone(),
                |value: &String, columns: &mut [Utf32String]| {
                    columns[0] = Utf32String::from(value.as_str());
                },
            );
        }

        nucleo.tick(10);

        Ok(Self {
            search_root,
            query: String::new(),
            nucleo,
            _injector: injector,
            selected_index: 0,
            scroll_offset: 0,
        })
    }

    /// Update pattern from current query and run a tick.
    pub fn update_pattern(&mut self) {
        self.nucleo.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            false,
        );
        self.nucleo.tick(10);
    }

    /// Number of matched items in current snapshot.
    pub fn matched_count(&self) -> u32 {
        self.nucleo.snapshot().matched_item_count()
    }

    /// Clamp selected_index to valid range.
    pub fn clamp_selection(&mut self) {
        let n = self.matched_count() as usize;
        if n == 0 {
            self.selected_index = 0;
            return;
        }
        if self.selected_index >= n {
            self.selected_index = n.saturating_sub(1);
        }
    }

    /// Get the path string of the currently selected match, if any.
    pub fn selected_path(&self) -> Option<String> {
        let snapshot = self.nucleo.snapshot();
        let n = snapshot.matched_item_count();
        if n == 0 || self.selected_index as u32 >= n {
            return None;
        }
        snapshot
            .get_matched_item(self.selected_index as u32)
            .map(|item| item.data.clone())
    }
}
