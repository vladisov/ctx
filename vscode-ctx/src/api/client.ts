import * as http from "http";
import {
  Pack,
  PackItem,
  RenderResult,
  CreatePackRequest,
  CreatePackResponse,
  AddArtifactRequest,
  AddArtifactResponse,
  DeleteResponse,
  ApiError,
  SuggestRequest,
  SuggestResponse,
} from "./types";
import { getConfig } from "../config";

export class CtxApiClient {
  private get baseUrl(): string {
    const config = getConfig();
    const host = config.get<string>("server.host", "127.0.0.1");
    const port = config.get<number>("server.port", 17373);
    return `http://${host}:${port}`;
  }

  // Pack Operations

  async listPacks(): Promise<Pack[]> {
    return this.get<Pack[]>("/api/packs");
  }

  async getPack(name: string): Promise<Pack> {
    return this.get<Pack>(`/api/packs/${encodeURIComponent(name)}`);
  }

  async createPack(request: CreatePackRequest): Promise<CreatePackResponse> {
    return this.post("/api/packs", request);
  }

  async deletePack(name: string): Promise<DeleteResponse> {
    return this.delete(`/api/packs/${encodeURIComponent(name)}`);
  }

  async renderPack(name: string): Promise<RenderResult> {
    return this.get<RenderResult>(
      `/api/packs/${encodeURIComponent(name)}/render`
    );
  }

  // Artifact Operations

  async listArtifacts(packName: string): Promise<PackItem[]> {
    return this.get<PackItem[]>(
      `/api/packs/${encodeURIComponent(packName)}/artifacts`
    );
  }

  async addArtifact(
    packName: string,
    artifact: AddArtifactRequest
  ): Promise<AddArtifactResponse> {
    return this.post(
      `/api/packs/${encodeURIComponent(packName)}/artifacts`,
      artifact
    );
  }

  async removeArtifact(
    packName: string,
    artifactId: string
  ): Promise<DeleteResponse> {
    return this.delete(
      `/api/packs/${encodeURIComponent(packName)}/artifacts/${encodeURIComponent(artifactId)}`
    );
  }

  // Suggestion Operations

  async getSuggestions(request: SuggestRequest): Promise<SuggestResponse> {
    const params = new URLSearchParams({ file: request.file });
    if (request.pack) {
      params.set("pack", request.pack);
    }
    if (request.max_results) {
      params.set("max_results", String(request.max_results));
    }
    return this.get<SuggestResponse>(`/api/suggest?${params}`);
  }

  // Health Check

  async healthCheck(): Promise<boolean> {
    try {
      await this.get("/");
      return true;
    } catch {
      return false;
    }
  }

  // HTTP Methods

  private get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path);
  }

  private post<T>(path: string, body: object): Promise<T> {
    return this.request<T>("POST", path, body);
  }

  private delete<T>(path: string): Promise<T> {
    return this.request<T>("DELETE", path);
  }

  private request<T>(method: string, path: string, body?: object): Promise<T> {
    return new Promise((resolve, reject) => {
      const url = new URL(path, this.baseUrl);

      const options: http.RequestOptions = {
        hostname: url.hostname,
        port: url.port,
        path: url.pathname + url.search,
        method,
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
      };

      const req = http.request(options, (res) => {
        let data = "";
        res.on("data", (chunk) => (data += chunk));
        res.on("end", () => {
          if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            try {
              resolve(JSON.parse(data));
            } catch {
              resolve(data as unknown as T);
            }
          } else {
            reject({
              error: data || "Request failed",
              status: res.statusCode || 500,
            } as ApiError);
          }
        });
      });

      req.on("error", (err) => {
        reject({ error: err.message, status: 0 } as ApiError);
      });

      req.setTimeout(10000, () => {
        req.destroy();
        reject({ error: "Request timeout", status: 0 } as ApiError);
      });

      if (body) {
        req.write(JSON.stringify(body));
      }
      req.end();
    });
  }
}
