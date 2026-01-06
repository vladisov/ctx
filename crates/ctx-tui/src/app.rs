use anyhow::Result;
use ctx_core::{Pack, render::RenderResult};
use ctx_storage::{Storage, PackItem};
use std::collections::HashMap;

pub enum Focus {
    PackList,
    Preview,
}

pub struct App {
    pub storage: Storage,
    pub packs: Vec<Pack>,
    pub selected_pack_index: usize,
    pub expanded_packs: Vec<String>, // Pack IDs that are expanded
    pub pack_artifacts: HashMap<String, Vec<PackItem>>, // Cache of pack artifacts
    pub focus: Focus,
    pub preview_result: Option<RenderResult>,
    pub status_message: Option<String>,
}

impl App {
    pub async fn new(storage: Storage) -> Result<Self> {
        let packs = storage.list_packs().await?;
        Ok(Self {
            storage,
            packs,
            selected_pack_index: 0,
            expanded_packs: Vec::new(),
            pack_artifacts: HashMap::new(),
            focus: Focus::PackList,
            preview_result: None,
            status_message: None,
        })
    }

    pub fn next(&mut self) {
        if !self.packs.is_empty() {
            self.selected_pack_index = (self.selected_pack_index + 1) % self.packs.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.packs.is_empty() {
            self.selected_pack_index = if self.selected_pack_index == 0 {
                self.packs.len() - 1
            } else {
                self.selected_pack_index - 1
            };
        }
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
}
