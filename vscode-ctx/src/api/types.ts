// TypeScript types matching ctx Rust types

export interface Pack {
  id: string;
  name: string;
  policies: RenderPolicy;
  created_at: number; // Unix timestamp
  updated_at: number;
}

export interface RenderPolicy {
  budget_tokens: number;
  ordering: OrderingStrategy;
}

export type OrderingStrategy = "PriorityThenTime";

export interface Artifact {
  id: string;
  type: string;
  source_uri: string;
  content_hash: string | null;
  metadata: ArtifactMetadata;
  token_estimate: number;
  created_at: number;
  // Type-specific fields (flattened in JSON)
  path?: string;
  content?: string;
  pattern?: string;
  start?: number;
  end?: number;
  base?: string;
  head?: string;
}

export interface ArtifactMetadata {
  size_bytes: number;
  mime_type?: string;
}

export interface PackItem {
  pack_id: string;
  artifact: Artifact;
  priority: number;
  added_at: number;
}

export interface RenderResult {
  pack: string;
  token_estimate: number;
  content: string;
}

export interface CreatePackRequest {
  name: string;
  budget_tokens?: number;
}

export interface CreatePackResponse {
  id: string;
  name: string;
  message: string;
}

export interface AddArtifactRequest {
  type: string;
  path?: string;
  content?: string;
  pattern?: string;
  start?: number;
  end?: number;
  base?: string;
  head?: string;
  priority?: number;
}

export interface AddArtifactResponse {
  artifact_id: string;
  message: string;
}

export interface DeleteResponse {
  message: string;
}

export interface ApiError {
  error: string;
  status: number;
}

// Suggestion types
export interface SuggestRequest {
  file: string;
  pack?: string;
  max_results?: number;
}

export interface SuggestResponse {
  file: string;
  suggestions: Suggestion[];
  elapsed_ms: number;
}

export interface Suggestion {
  path: string;
  score: number;
  reasons: SuggestionReason[];
}

export interface SuggestionReason {
  signal: string;
  description: string;
  contribution: number;
}
