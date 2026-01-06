use anyhow::Result;
use ctx_core::{Pack, render::RenderResult, ArtifactType};
use ctx_storage::{Storage, PackItem};
use ctx_sources::{SourceHandlerRegistry, SourceOptions};
use std::collections::HashMap;

pub enum Focus {
    PackList,
    Preview,
}

pub enum InputMode {
    Normal,
    AddingArtifact,
    ConfirmDeletePack,
}

pub enum PreviewMode {
    Stats,
    Content,
}

pub struct App {
    pub storage: Storage,
    pub packs: Vec<Pack>,
    pub selected_pack_index: usize,
    pub selected_artifact_index: Option<usize>, // Index within expanded pack's artifacts
    pub expanded_packs: Vec<String>, // Pack IDs that are expanded
    pub pack_artifacts: HashMap<String, Vec<PackItem>>, // Cache of pack artifacts
    pub focus: Focus,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub preview_result: Option<RenderResult>,
    pub preview_mode: PreviewMode,
    pub content_scroll: usize,
    pub status_message: Option<String>,
}

impl App {
    pub async fn new(storage: Storage) -> Result<Self> {
        let packs = storage.list_packs().await?;
        Ok(Self {
            storage,
            packs,
            selected_pack_index: 0,
            selected_artifact_index: None,
            expanded_packs: Vec::new(),
            pack_artifacts: HashMap::new(),
            focus: Focus::PackList,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            preview_result: None,
            preview_mode: PreviewMode::Stats,
            content_scroll: 0,
            status_message: None,
        })
    }

    pub fn next(&mut self) {
        if self.packs.is_empty() {
            return;
        }

        // If pack is expanded and has artifacts, navigate within artifacts
        if let Some(pack) = self.packs.get(self.selected_pack_index) {
            if self.is_expanded(&pack.id) {
                if let Some(artifacts) = self.pack_artifacts.get(&pack.id) {
                    if !artifacts.is_empty() {
                        if let Some(idx) = self.selected_artifact_index {
                            if idx < artifacts.len() - 1 {
                                self.selected_artifact_index = Some(idx + 1);
                                return;
                            }
                        } else {
                            self.selected_artifact_index = Some(0);
                            return;
                        }
                    }
                }
            }
        }

        // Move to next pack
        self.selected_artifact_index = None;
        self.selected_pack_index = (self.selected_pack_index + 1) % self.packs.len();
    }

    pub fn previous(&mut self) {
        if self.packs.is_empty() {
            return;
        }

        // If artifact is selected, go up
        if let Some(idx) = self.selected_artifact_index {
            if idx > 0 {
                self.selected_artifact_index = Some(idx - 1);
            } else {
                self.selected_artifact_index = None; // Go back to pack
            }
            return;
        }

        // Move to previous pack
        self.selected_pack_index = if self.selected_pack_index == 0 {
            self.packs.len() - 1
        } else {
            self.selected_pack_index - 1
        };
    }

    pub async fn toggle_expand(&mut self) -> Result<()> {
        if let Some(pack) = self.packs.get(self.selected_pack_index) {
            let pack_id = pack.id.clone();
            if let Some(pos) = self.expanded_packs.iter().position(|id| id == &pack_id) {
                self.expanded_packs.remove(pos);
            } else {
                // Load artifacts if not already cached
                if !self.pack_artifacts.contains_key(&pack_id) {
                    match self.storage.get_pack_artifacts(&pack_id).await {
                        Ok(artifacts) => {
                            self.pack_artifacts.insert(pack_id.clone(), artifacts);
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to load sources: {}", e));
                            return Ok(());
                        }
                    }
                }
                self.expanded_packs.push(pack_id);
            }
        }
        Ok(())
    }

    pub fn is_expanded(&self, pack_id: &str) -> bool {
        self.expanded_packs.contains(&pack_id.to_string())
    }

    pub async fn preview(&mut self) -> Result<()> {
        if let Some(pack) = self.packs.get(self.selected_pack_index) {
            let renderer = ctx_engine::Renderer::new(self.storage.clone());
            match renderer.render_pack(&pack.id, None).await {
                Ok(result) => {
                    self.preview_result = Some(result);
                    self.status_message = Some("Preview generated".to_string());
                }
                Err(e) => {
                    self.status_message = Some(format!("Preview failed: {}", e));
                }
            }
        }
        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.packs = self.storage.list_packs().await?;
        self.status_message = Some("Refreshed".to_string());
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

    pub fn scroll_content_up(&mut self) {
        self.content_scroll = self.content_scroll.saturating_sub(1);
    }

    pub fn scroll_content_down(&mut self) {
        self.content_scroll = self.content_scroll.saturating_add(1);
    }

    pub fn start_add_artifact(&mut self) {
        self.input_mode = InputMode::AddingArtifact;
        self.input_buffer.clear();
    }

    pub fn start_delete_pack(&mut self) {
        self.input_mode = InputMode::ConfirmDeletePack;
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    pub fn input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub async fn confirm_add_artifact(&mut self) -> Result<()> {
        if let Some(pack) = self.packs.get(self.selected_pack_index) {
            let uri = self.input_buffer.trim().to_string();
            if uri.is_empty() {
                self.status_message = Some("URI cannot be empty".to_string());
                self.cancel_input();
                return Ok(());
            }

            // Parse URI into artifact
            let registry = SourceHandlerRegistry::new();
            let options = SourceOptions {
                range: None,
                max_files: None,
                exclude: Vec::new(),
                recursive: false,
                priority: 0,
            };

            match registry.parse(&uri, options).await {
                Ok(artifact) => {
                    // Check if it's a collection
                    let is_collection = matches!(
                        artifact.artifact_type,
                        ArtifactType::CollectionMdDir { .. } | ArtifactType::CollectionGlob { .. }
                    );

                    let result: Result<()> = async {
                        if is_collection {
                            self.storage.create_artifact(&artifact).await?;
                            self.storage.add_artifact_to_pack(&pack.id, &artifact.id, 0).await?;
                        } else {
                            let content = registry.load(&artifact).await?;
                            self.storage
                                .add_artifact_to_pack_with_content(&pack.id, &artifact, &content, 0)
                                .await?;
                        }
                        Ok(())
                    }.await;

                    match result {
                        Ok(_) => {
                            self.status_message = Some(format!("Added: {}", uri));
                            // Invalidate cache
                            self.pack_artifacts.remove(&pack.id);
                            // Reload if expanded
                            if self.is_expanded(&pack.id) {
                                if let Ok(artifacts) = self.storage.get_pack_artifacts(&pack.id).await {
                                    self.pack_artifacts.insert(pack.id.clone(), artifacts);
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to add: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.status_message = Some(format!("Parse error: {}", e));
                }
            }
        }
        self.cancel_input();
        Ok(())
    }

    pub async fn delete_artifact(&mut self) -> Result<()> {
        if let Some(artifact_idx) = self.selected_artifact_index {
            if let Some(pack) = self.packs.get(self.selected_pack_index) {
                if let Some(artifacts) = self.pack_artifacts.get(&pack.id) {
                    if let Some(item) = artifacts.get(artifact_idx) {
                        let artifact_id = item.artifact.id.clone();
                        let uri = item.artifact.source_uri.clone();

                        match self.storage.remove_artifact_from_pack(&pack.id, &artifact_id).await {
                            Ok(_) => {
                                self.status_message = Some(format!("Removed: {}", uri));
                                // Reload artifacts
                                if let Ok(new_artifacts) = self.storage.get_pack_artifacts(&pack.id).await {
                                    self.pack_artifacts.insert(pack.id.clone(), new_artifacts);
                                    self.selected_artifact_index = None;
                                }
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Failed to remove: {}", e));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn confirm_delete_pack(&mut self) -> Result<()> {
        if let Some(pack) = self.packs.get(self.selected_pack_index) {
            let pack_id = pack.id.clone();
            let pack_name = pack.name.clone();

            match self.storage.delete_pack(&pack_id).await {
                Ok(_) => {
                    self.status_message = Some(format!("Deleted pack: {}", pack_name));
                    self.pack_artifacts.remove(&pack_id);
                    self.packs = self.storage.list_packs().await?;
                    if self.selected_pack_index >= self.packs.len() && !self.packs.is_empty() {
                        self.selected_pack_index = self.packs.len() - 1;
                    }
                    self.selected_artifact_index = None;
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to delete: {}", e));
                }
            }
        }
        self.cancel_input();
        Ok(())
    }
}
