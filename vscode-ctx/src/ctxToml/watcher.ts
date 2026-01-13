import * as vscode from "vscode";

export class CtxTomlWatcher implements vscode.Disposable {
  private watcher?: vscode.FileSystemWatcher;
  private _onCtxTomlChange = new vscode.EventEmitter<vscode.Uri>();
  private _onCtxTomlCreated = new vscode.EventEmitter<vscode.Uri>();
  private _onCtxTomlDeleted = new vscode.EventEmitter<vscode.Uri>();
  private disposables: vscode.Disposable[] = [];

  readonly onCtxTomlChange = this._onCtxTomlChange.event;
  readonly onCtxTomlCreated = this._onCtxTomlCreated.event;
  readonly onCtxTomlDeleted = this._onCtxTomlDeleted.event;

  constructor() {
    this.setupWatcher();
  }

  private setupWatcher(): void {
    // Watch for ctx.toml in all workspace folders
    this.watcher = vscode.workspace.createFileSystemWatcher("**/ctx.toml");

    this.disposables.push(
      this.watcher.onDidChange((uri) => {
        this._onCtxTomlChange.fire(uri);
      })
    );

    this.disposables.push(
      this.watcher.onDidCreate((uri) => {
        this._onCtxTomlCreated.fire(uri);
        vscode.commands.executeCommand("setContext", "ctx.hasCtxToml", true);
      })
    );

    this.disposables.push(
      this.watcher.onDidDelete((uri) => {
        this._onCtxTomlDeleted.fire(uri);
        this.checkCtxTomlExists();
      })
    );
  }

  async checkCtxTomlExists(): Promise<boolean> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
      await vscode.commands.executeCommand("setContext", "ctx.hasCtxToml", false);
      return false;
    }

    for (const folder of workspaceFolders) {
      const ctxTomlPath = vscode.Uri.joinPath(folder.uri, "ctx.toml");
      try {
        await vscode.workspace.fs.stat(ctxTomlPath);
        await vscode.commands.executeCommand("setContext", "ctx.hasCtxToml", true);
        return true;
      } catch {
        // File doesn't exist in this folder
      }
    }

    await vscode.commands.executeCommand("setContext", "ctx.hasCtxToml", false);
    return false;
  }

  async findCtxToml(): Promise<vscode.Uri | undefined> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
      return undefined;
    }

    for (const folder of workspaceFolders) {
      const ctxTomlPath = vscode.Uri.joinPath(folder.uri, "ctx.toml");
      try {
        await vscode.workspace.fs.stat(ctxTomlPath);
        return ctxTomlPath;
      } catch {
        // Continue to next folder
      }
    }

    return undefined;
  }

  dispose(): void {
    this.watcher?.dispose();
    this._onCtxTomlChange.dispose();
    this._onCtxTomlCreated.dispose();
    this._onCtxTomlDeleted.dispose();
    this.disposables.forEach((d) => d.dispose());
  }
}
