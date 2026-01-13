import * as vscode from "vscode";
import { CtxApiClient } from "../api/client";
import { Pack, PackItem, Artifact } from "../api/types";

type TreeItem = PackTreeItem | ArtifactTreeItem;

export class PackTreeItem extends vscode.TreeItem {
  constructor(public readonly pack: Pack) {
    super(pack.name, vscode.TreeItemCollapsibleState.Collapsed);

    this.id = pack.id;
    this.description = `${pack.policies.budget_tokens.toLocaleString()} tokens`;
    this.tooltip = new vscode.MarkdownString(
      `**${pack.name}**\n\n` +
        `- ID: \`${pack.id}\`\n` +
        `- Budget: ${pack.policies.budget_tokens.toLocaleString()} tokens\n` +
        `- Created: ${new Date(pack.created_at * 1000).toLocaleString()}`
    );
    this.contextValue = "pack";
    this.iconPath = new vscode.ThemeIcon("package");
  }
}

export class ArtifactTreeItem extends vscode.TreeItem {
  constructor(
    public readonly artifact: Artifact,
    public readonly packName: string,
    public readonly priority: number
  ) {
    super(ArtifactTreeItem.getLabel(artifact), vscode.TreeItemCollapsibleState.None);

    this.id = artifact.id;
    this.description = `priority: ${priority}`;
    this.tooltip = new vscode.MarkdownString(
      `**${artifact.source_uri}**\n\n` +
        `- Type: \`${artifact.type}\`\n` +
        `- Priority: ${priority}\n` +
        `- Tokens: ~${artifact.token_estimate}`
    );
    this.contextValue = "artifact";
    this.iconPath = ArtifactTreeItem.getIcon(artifact);

    // Make file artifacts clickable to open the file
    if (artifact.path) {
      this.command = {
        command: "vscode.open",
        title: "Open File",
        arguments: [vscode.Uri.file(artifact.path)],
      };
    }
  }

  private static getLabel(artifact: Artifact): string {
    if (artifact.path) {
      return artifact.path.split("/").pop() || artifact.path;
    }
    if (artifact.pattern) {
      return artifact.pattern;
    }
    if (artifact.content) {
      const preview = artifact.content.substring(0, 30);
      return preview + (artifact.content.length > 30 ? "..." : "");
    }
    if (artifact.base) {
      return `diff: ${artifact.base}..${artifact.head || "HEAD"}`;
    }
    return artifact.source_uri;
  }

  private static getIcon(artifact: Artifact): vscode.ThemeIcon {
    switch (artifact.type) {
      case "file":
      case "file_range":
      case "markdown":
        return new vscode.ThemeIcon("file");
      case "collection_glob":
      case "collection_md_dir":
        return new vscode.ThemeIcon("file-directory");
      case "text":
        return new vscode.ThemeIcon("note");
      case "git_diff":
        return new vscode.ThemeIcon("git-compare");
      default:
        return new vscode.ThemeIcon("symbol-misc");
    }
  }
}

export class PacksTreeProvider implements vscode.TreeDataProvider<TreeItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<TreeItem | undefined | null | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private api: CtxApiClient;
  private packsCache: Map<string, Pack> = new Map();

  constructor() {
    this.api = new CtxApiClient();
  }

  refresh(): void {
    this.packsCache.clear();
    this._onDidChangeTreeData.fire();
  }

  refreshPack(packId: string): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: TreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: TreeItem): Promise<TreeItem[]> {
    if (!element) {
      // Root level - return packs
      try {
        const packs = await this.api.listPacks();
        return packs.map((pack) => {
          this.packsCache.set(pack.id, pack);
          return new PackTreeItem(pack);
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        vscode.window.showErrorMessage(`Failed to load packs: ${message}`);
        return [];
      }
    }

    if (element instanceof PackTreeItem) {
      // Pack level - return artifacts
      try {
        const items = await this.api.listArtifacts(element.pack.name);
        return items.map(
          (item) => new ArtifactTreeItem(item.artifact, element.pack.name, item.priority)
        );
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        vscode.window.showWarningMessage(`Failed to load artifacts: ${message}`);
        return [];
      }
    }

    return [];
  }

  getParent(element: TreeItem): vscode.ProviderResult<TreeItem> {
    if (element instanceof ArtifactTreeItem) {
      const pack = this.packsCache.get(element.packName);
      if (pack) {
        return new PackTreeItem(pack);
      }
    }
    return null;
  }
}
