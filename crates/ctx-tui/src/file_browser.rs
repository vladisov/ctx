use anyhow::Result;
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub enum ArtifactTypeSelection {
    File,
    Glob,
    GitDiff,
}

impl ArtifactTypeSelection {
    pub fn label(&self) -> &str {
        match self {
            Self::File => "file:",
            Self::Glob => "glob:",
            Self::GitDiff => "git:diff",
        }
    }
}

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_hidden: bool,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub show_hidden: bool,
    pub artifact_type: ArtifactTypeSelection,
    pub scroll_offset: usize,
}

impl FileBrowser {
    pub fn new(start_dir: Option<PathBuf>) -> Result<Self> {
        let current_dir = start_dir
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));

        let mut browser = Self {
            current_dir,
            entries: Vec::new(),
            selected_index: 0,
            show_hidden: false,
            artifact_type: ArtifactTypeSelection::File,
            scroll_offset: 0,
        };
        browser.load_entries()?;
        Ok(browser)
    }

    pub fn load_entries(&mut self) -> Result<()> {
        self.entries.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;

        // Add parent directory entry
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                is_hidden: false,
            });
        }

        // Read and filter directory contents
        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            self.entries.extend(read_dir.flatten().filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_hidden = name.starts_with('.');
                if !self.show_hidden && is_hidden {
                    return None;
                }
                let path = entry.path();
                Some(FileEntry {
                    name,
                    is_dir: path.is_dir(),
                    path,
                    is_hidden,
                })
            }));
        }

        // Sort: ".." first, then directories, then files alphabetically
        self.entries.sort_by(|a, b| match (&a.name[..], &b.name[..]) {
            ("..", _) => Ordering::Less,
            (_, "..") => Ordering::Greater,
            _ => match (a.is_dir, b.is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            },
        });
        Ok(())
    }

    pub fn next(&mut self, visible_height: usize) {
        if self.entries.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1).min(self.entries.len() - 1);
        if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }
    }

    pub fn previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
    }

    pub fn enter_selected(&mut self) -> Result<bool> {
        let Some(entry) = self.entries.get(self.selected_index) else {
            return Ok(false);
        };
        if !entry.is_dir {
            return Ok(false);
        }
        self.current_dir = entry.path.clone();
        self.load_entries()?;
        Ok(true)
    }

    pub fn go_up(&mut self) -> Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.load_entries()?;
        }
        Ok(())
    }

    pub fn toggle_hidden(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.load_entries()
    }

    pub fn cycle_artifact_type(&mut self) {
        self.artifact_type = match self.artifact_type {
            ArtifactTypeSelection::File => ArtifactTypeSelection::Glob,
            ArtifactTypeSelection::Glob => ArtifactTypeSelection::GitDiff,
            ArtifactTypeSelection::GitDiff => ArtifactTypeSelection::File,
        };
    }

    pub fn get_selected_uri(&self) -> Option<String> {
        let entry = self.entries.get(self.selected_index)?;
        if entry.name == ".." {
            return None;
        }

        match self.artifact_type {
            ArtifactTypeSelection::File | ArtifactTypeSelection::Glob => {
                let path = entry.path.display();
                if entry.is_dir {
                    Some(format!("glob:{path}/**/*"))
                } else {
                    Some(format!("file:{path}"))
                }
            }
            ArtifactTypeSelection::GitDiff => Some("git:diff --base=main".to_string()),
        }
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }
}
