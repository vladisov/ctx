use anyhow::Result;
use crate::file_browser::FileBrowser;
use ctx_core::{ArtifactType, Pack, RenderPolicy, OrderingStrategy, render::RenderResult};
use ctx_sources::{SourceHandlerRegistry, SourceOptions};
use ctx_storage::{PackItem, Storage};
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
pub enum Focus { PackList, Preview }

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    BrowsingFiles,
    AddingArtifact,
    CreatingPack,
    EditingBudget,
    ConfirmDeletePack,
    ShowingHelp,
}

#[derive(Clone, PartialEq)]
pub enum PreviewMode { Stats, Content }

pub struct App {
    pub storage: Storage,
    pub packs: Vec<Pack>,
    pub selected_pack_index: usize,
    pub selected_artifact_index: Option<usize>,
    pub expanded_packs: Vec<String>,
    pub pack_artifacts: HashMap<String, Vec<PackItem>>,
    pub artifact_content: Option<String>,
    pub focus: Focus,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub file_browser: Option<FileBrowser>,
    pub preview_result: Option<RenderResult>,
    pub preview_mode: PreviewMode,
    pub content_scroll: usize,
    pub status_message: Option<String>,
    pub loading_message: Option<String>,
}

impl App {
    pub async fn new(storage: Storage) -> Result<Self> {
        Ok(Self {
            packs: storage.list_packs().await?,
            storage,
            selected_pack_index: 0,
            selected_artifact_index: None,
            expanded_packs: Vec::new(),
            pack_artifacts: HashMap::new(),
            artifact_content: None,
            focus: Focus::PackList,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            file_browser: None,
            preview_result: None,
            preview_mode: PreviewMode::Stats,
            content_scroll: 0,
            status_message: None,
            loading_message: None,
        })
    }

    fn clear_artifact_state(&mut self) {
        self.artifact_content = None;
        self.content_scroll = 0;
    }

    fn selected_pack(&self) -> Option<&Pack> {
        self.packs.get(self.selected_pack_index)
    }

    pub fn next(&mut self) {
        if self.packs.is_empty() { return; }

        // Navigate within artifacts if pack is expanded
        if let Some(pack) = self.selected_pack() {
            if self.is_expanded(&pack.id) {
                if let Some(artifacts) = self.pack_artifacts.get(&pack.id) {
                    if !artifacts.is_empty() {
                        match self.selected_artifact_index {
                            Some(idx) if idx < artifacts.len() - 1 => {
                                self.selected_artifact_index = Some(idx + 1);
                                self.clear_artifact_state();
                                return;
                            }
                            None => {
                                self.selected_artifact_index = Some(0);
                                self.clear_artifact_state();
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        self.selected_artifact_index = None;
        self.clear_artifact_state();
        self.selected_pack_index = (self.selected_pack_index + 1) % self.packs.len();
    }

    pub fn previous(&mut self) {
        if self.packs.is_empty() { return; }

        if let Some(idx) = self.selected_artifact_index {
            self.selected_artifact_index = if idx > 0 { Some(idx - 1) } else { None };
            self.clear_artifact_state();
            return;
        }

        self.selected_pack_index = self.selected_pack_index
            .checked_sub(1)
            .unwrap_or(self.packs.len() - 1);
        self.clear_artifact_state();
    }

    pub async fn toggle_expand(&mut self) -> Result<()> {
        let Some(pack) = self.selected_pack() else { return Ok(()) };
        let pack_id = pack.id.clone();

        if let Some(pos) = self.expanded_packs.iter().position(|id| id == &pack_id) {
            self.expanded_packs.remove(pos);
        } else {
            if !self.pack_artifacts.contains_key(&pack_id) {
                match self.storage.get_pack_artifacts(&pack_id).await {
                    Ok(artifacts) => { self.pack_artifacts.insert(pack_id.clone(), artifacts); }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to load sources: {e}"));
                        return Ok(());
                    }
                }
            }
            self.expanded_packs.push(pack_id);
        }
        Ok(())
    }

    pub fn is_expanded(&self, pack_id: &str) -> bool {
        self.expanded_packs.iter().any(|id| id == pack_id)
    }

    pub async fn preview(&mut self) -> Result<()> {
        if let Some(idx) = self.selected_artifact_index {
            return self.load_artifact_content(idx).await;
        }

        let Some(pack_id) = self.selected_pack().map(|p| p.id.clone()) else { return Ok(()) };
        self.loading_message = Some("Generating preview...".into());

        let result = ctx_engine::Renderer::new(self.storage.clone())
            .render_pack(&pack_id, None).await;

        match result {
            Ok(r) => {
                self.preview_result = Some(r);
                self.status_message = Some("Preview generated".into());
            }
            Err(e) => self.status_message = Some(format!("Preview failed: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }

    pub async fn load_artifact_content(&mut self, idx: usize) -> Result<()> {
        let Some(pack) = self.selected_pack() else { return Ok(()) };
        let Some(artifacts) = self.pack_artifacts.get(&pack.id) else { return Ok(()) };
        let Some(item) = artifacts.get(idx) else { return Ok(()) };

        self.loading_message = Some("Loading artifact content...".into());

        match SourceHandlerRegistry::new().load(&item.artifact).await {
            Ok(content) => {
                self.artifact_content = Some(content);
                self.content_scroll = 0;
                self.preview_mode = PreviewMode::Content;
                self.status_message = Some(format!("Loaded: {}", item.artifact.source_uri));
            }
            Err(e) => self.status_message = Some(format!("Failed to load: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.loading_message = Some("Refreshing...".into());
        self.packs = self.storage.list_packs().await?;
        self.loading_message = None;
        self.status_message = Some("Refreshed".into());
        Ok(())
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::PackList => Focus::Preview,
            Focus::Preview => Focus::PackList,
        };
    }

    pub fn toggle_preview_mode(&mut self) {
        self.preview_mode = match self.preview_mode {
            PreviewMode::Stats => PreviewMode::Content,
            PreviewMode::Content => PreviewMode::Stats,
        };
        self.content_scroll = 0;
    }

    pub fn scroll_content_up(&mut self) { self.content_scroll = self.content_scroll.saturating_sub(1); }
    pub fn scroll_content_down(&mut self) { self.content_scroll += 1; }
    pub fn scroll_page_up(&mut self) { self.content_scroll = self.content_scroll.saturating_sub(10); }
    pub fn scroll_page_down(&mut self) { self.content_scroll += 10; }

    pub fn start_add_artifact(&mut self) {
        match FileBrowser::new(None) {
            Ok(browser) => {
                self.file_browser = Some(browser);
                self.input_mode = InputMode::BrowsingFiles;
            }
            Err(_) => {
                self.input_mode = InputMode::AddingArtifact;
                self.input_buffer.clear();
                self.status_message = Some("File browser unavailable".into());
            }
        }
    }

    pub fn start_create_pack(&mut self) {
        self.input_mode = InputMode::CreatingPack;
        self.input_buffer.clear();
    }

    pub fn start_edit_budget(&mut self) {
        if let Some(pack) = self.selected_pack() {
            self.input_buffer = pack.policies.budget_tokens.to_string();
            self.input_mode = InputMode::EditingBudget;
        }
    }

    pub fn start_delete_pack(&mut self) { self.input_mode = InputMode::ConfirmDeletePack; }

    pub fn toggle_help(&mut self) {
        self.input_mode = match self.input_mode {
            InputMode::ShowingHelp => InputMode::Normal,
            InputMode::Normal => InputMode::ShowingHelp,
            _ => return,
        };
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.file_browser = None;
    }

    pub fn input_char(&mut self, c: char) { self.input_buffer.push(c); }
    pub fn input_backspace(&mut self) { self.input_buffer.pop(); }

    // File browser operations
    pub fn browser_next(&mut self, h: usize) { if let Some(b) = &mut self.file_browser { b.next(h); } }
    pub fn browser_previous(&mut self) { if let Some(b) = &mut self.file_browser { b.previous(); } }
    pub fn browser_enter(&mut self) -> Result<()> { self.file_browser.as_mut().map(|b| b.enter_selected()).transpose()?; Ok(()) }
    pub fn browser_go_up(&mut self) -> Result<()> { self.file_browser.as_mut().map(|b| b.go_up()).transpose()?; Ok(()) }
    pub fn browser_toggle_hidden(&mut self) -> Result<()> { self.file_browser.as_mut().map(|b| b.toggle_hidden()).transpose()?; Ok(()) }
    pub fn browser_cycle_type(&mut self) { if let Some(b) = &mut self.file_browser { b.cycle_artifact_type(); } }

    pub fn browser_switch_to_text_input(&mut self) {
        self.file_browser = None;
        self.input_mode = InputMode::AddingArtifact;
        self.input_buffer.clear();
    }

    pub async fn browser_confirm_selection(&mut self) -> Result<()> {
        if self.file_browser.as_ref().map(|b| b.is_text_mode()).unwrap_or(false) {
            self.browser_switch_to_text_input();
            self.status_message = Some("Text mode: type your content".into());
            return Ok(());
        }

        let uri = self.file_browser.as_ref().and_then(|b| b.get_selected_uri());
        if let Some(uri) = uri {
            self.input_buffer = uri;
            self.file_browser = None;
            self.input_mode = InputMode::Normal;
            self.confirm_add_artifact().await?;
        } else {
            self.status_message = Some("Cannot select (use Enter to navigate)".into());
        }
        Ok(())
    }

    pub async fn confirm_add_artifact(&mut self) -> Result<()> {
        let Some(pack_id) = self.selected_pack().map(|p| p.id.clone()) else {
            self.cancel_input();
            return Ok(());
        };

        let uri = self.input_buffer.trim().to_string();
        if uri.is_empty() {
            self.status_message = Some("URI cannot be empty".into());
            self.cancel_input();
            return Ok(());
        }

        self.cancel_input();
        self.loading_message = Some("Adding artifact...".into());

        let registry = SourceHandlerRegistry::new();
        let options = SourceOptions { range: None, max_files: None, exclude: Vec::new(), recursive: false, priority: 0 };

        match registry.parse(&uri, options).await {
            Ok(artifact) => {
                let is_collection = matches!(artifact.artifact_type,
                    ArtifactType::CollectionMdDir { .. } | ArtifactType::CollectionGlob { .. });

                let result: Result<()> = async {
                    if is_collection {
                        self.storage.create_artifact(&artifact).await.map_err(|e| anyhow::anyhow!("{e}"))?;
                        self.storage.add_artifact_to_pack(&pack_id, &artifact.id, 0).await.map_err(|e| anyhow::anyhow!("{e}"))
                    } else {
                        let content = registry.load(&artifact).await?;
                        self.storage.add_artifact_to_pack_with_content(&pack_id, &artifact, &content, 0).await
                            .map(|_| ()).map_err(|e| anyhow::anyhow!("{e}"))
                    }
                }.await;

                match result {
                    Ok(_) => {
                        self.status_message = Some(format!("Added: {uri}"));
                        self.pack_artifacts.remove(&pack_id);
                        if self.is_expanded(&pack_id) {
                            if let Ok(a) = self.storage.get_pack_artifacts(&pack_id).await {
                                self.pack_artifacts.insert(pack_id, a);
                            }
                        }
                    }
                    Err(e) => self.status_message = Some(format!("Failed: {e}")),
                }
            }
            Err(e) => self.status_message = Some(format!("Parse error: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }

    pub async fn delete_artifact(&mut self) -> Result<()> {
        let Some(idx) = self.selected_artifact_index else { return Ok(()) };
        let Some(pack) = self.selected_pack() else { return Ok(()) };
        let Some(artifacts) = self.pack_artifacts.get(&pack.id) else { return Ok(()) };
        let Some(item) = artifacts.get(idx) else { return Ok(()) };

        let (pack_id, artifact_id, uri) = (pack.id.clone(), item.artifact.id.clone(), item.artifact.source_uri.clone());

        self.loading_message = Some("Deleting...".into());
        match self.storage.remove_artifact_from_pack(&pack_id, &artifact_id).await {
            Ok(_) => {
                self.status_message = Some(format!("Removed: {uri}"));
                if let Ok(a) = self.storage.get_pack_artifacts(&pack_id).await {
                    self.pack_artifacts.insert(pack_id, a);
                    self.selected_artifact_index = None;
                }
            }
            Err(e) => self.status_message = Some(format!("Failed: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }

    pub async fn confirm_delete_pack(&mut self) -> Result<()> {
        let Some(pack) = self.selected_pack() else { self.cancel_input(); return Ok(()) };
        let (pack_id, pack_name) = (pack.id.clone(), pack.name.clone());

        self.cancel_input();
        self.loading_message = Some("Deleting pack...".into());

        match self.storage.delete_pack(&pack_id).await {
            Ok(_) => {
                self.status_message = Some(format!("Deleted: {pack_name}"));
                self.pack_artifacts.remove(&pack_id);
                self.packs = self.storage.list_packs().await?;
                if self.selected_pack_index >= self.packs.len() && !self.packs.is_empty() {
                    self.selected_pack_index = self.packs.len() - 1;
                }
                self.selected_artifact_index = None;
            }
            Err(e) => self.status_message = Some(format!("Failed: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }

    pub async fn confirm_create_pack(&mut self) -> Result<()> {
        let input = self.input_buffer.trim();
        if input.is_empty() {
            self.status_message = Some("Name cannot be empty".into());
            self.cancel_input();
            return Ok(());
        }

        let (name, budget) = match input.split_once(':') {
            Some((n, b)) => match b.trim().parse() {
                Ok(budget) => (n.trim().to_string(), budget),
                Err(_) => {
                    self.status_message = Some("Invalid budget".into());
                    self.cancel_input();
                    return Ok(());
                }
            },
            None => (input.to_string(), 128000),
        };

        self.cancel_input();
        self.loading_message = Some(format!("Creating '{name}'..."));

        let pack = Pack::new(name.clone(), RenderPolicy {
            budget_tokens: budget,
            ordering: OrderingStrategy::PriorityThenTime,
        });

        match self.storage.create_pack(&pack).await {
            Ok(_) => {
                self.status_message = Some(format!("Created: {name} ({budget} tokens)"));
                self.packs = self.storage.list_packs().await?;
                self.selected_pack_index = self.packs.iter().position(|p| p.id == pack.id).unwrap_or(0);
            }
            Err(e) => self.status_message = Some(format!("Failed: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }

    pub async fn confirm_edit_budget(&mut self) -> Result<()> {
        let Ok(new_budget) = self.input_buffer.trim().parse::<usize>() else {
            self.status_message = Some("Invalid budget".into());
            self.cancel_input();
            return Ok(());
        };

        let Some(pack) = self.packs.get_mut(self.selected_pack_index) else {
            self.cancel_input();
            return Ok(());
        };

        pack.policies.budget_tokens = new_budget;
        let pack_clone = pack.clone();

        self.cancel_input();
        self.loading_message = Some("Updating...".into());

        match self.storage.create_pack(&pack_clone).await {
            Ok(_) => {
                self.status_message = Some(format!("Budget: {new_budget}"));
                self.packs = self.storage.list_packs().await?;
            }
            Err(e) => self.status_message = Some(format!("Failed: {e}")),
        }
        self.loading_message = None;
        Ok(())
    }
}
