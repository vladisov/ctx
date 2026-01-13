import * as vscode from "vscode";
import * as cp from "child_process";
import { getConfig } from "../config";
import { PacksTreeProvider, PackTreeItem } from "../views/PacksTreeProvider";
import { CtxApiClient } from "../api/client";

export function registerSyncCommands(
  context: vscode.ExtensionContext,
  treeProvider: PacksTreeProvider
): void {
  // Helper: Run ctx CLI command
  function runCtxCommand(args: string[], cwd: string): Promise<string> {
    const ctxPath = getConfig().get<string>("ctxBinaryPath") || "ctx";
    return new Promise((resolve, reject) => {
      cp.exec(`"${ctxPath}" ${args.join(" ")}`, { cwd }, (error, stdout, stderr) => {
        if (error) {
          reject(new Error(stderr || error.message));
        } else {
          resolve(stdout);
        }
      });
    });
  }

  // Helper: Find ctx.toml in workspace
  async function findCtxToml(): Promise<vscode.Uri | undefined> {
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
        // File doesn't exist in this folder
      }
    }

    return undefined;
  }

  // Sync from ctx.toml
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.syncFromToml", async () => {
      const ctxTomlUri = await findCtxToml();
      if (!ctxTomlUri) {
        vscode.window.showWarningMessage("No ctx.toml found in workspace");
        return;
      }

      const workspaceFolder = vscode.workspace.getWorkspaceFolder(ctxTomlUri);
      if (!workspaceFolder) {
        return;
      }

      try {
        await vscode.window.withProgress(
          {
            location: vscode.ProgressLocation.Notification,
            title: "Syncing packs from ctx.toml...",
            cancellable: false,
          },
          async () => {
            await runCtxCommand(["pack", "sync"], workspaceFolder.uri.fsPath);
            vscode.window.showInformationMessage("Packs synced from ctx.toml");
            treeProvider.refresh();
          }
        );
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        vscode.window.showErrorMessage(`Sync failed: ${message}`);
      }
    })
  );

  // Save pack to ctx.toml
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.saveToToml", async (item?: PackTreeItem) => {
      const ctxTomlUri = await findCtxToml();
      if (!ctxTomlUri) {
        const create = await vscode.window.showWarningMessage(
          "No ctx.toml found. Initialize one?",
          "Initialize"
        );
        if (create === "Initialize") {
          await vscode.commands.executeCommand("ctx.initCtxToml");
        }
        return;
      }

      const workspaceFolder = vscode.workspace.getWorkspaceFolder(ctxTomlUri);
      if (!workspaceFolder) {
        return;
      }

      let packName = item instanceof PackTreeItem ? item.pack.name : undefined;
      if (!packName) {
        const api = new CtxApiClient();
        const packs = await api.listPacks();
        const selected = await vscode.window.showQuickPick(
          packs.map((p) => ({ label: p.name, pack: p })),
          { placeHolder: "Select pack to save" }
        );
        packName = selected?.pack.name;
      }

      if (!packName) {
        return;
      }

      try {
        await runCtxCommand(["pack", "save", packName], workspaceFolder.uri.fsPath);
        vscode.window.showInformationMessage(`Saved ${packName} to ctx.toml`);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        vscode.window.showErrorMessage(`Save failed: ${message}`);
      }
    })
  );

  // Initialize ctx.toml
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.initCtxToml", async () => {
      const workspaceFolders = vscode.workspace.workspaceFolders;
      if (!workspaceFolders || workspaceFolders.length === 0) {
        vscode.window.showErrorMessage("No workspace folder open");
        return;
      }

      let targetFolder = workspaceFolders[0];
      if (workspaceFolders.length > 1) {
        const selected = await vscode.window.showQuickPick(
          workspaceFolders.map((f) => ({ label: f.name, folder: f })),
          { placeHolder: "Select folder to initialize ctx.toml" }
        );
        if (!selected) {
          return;
        }
        targetFolder = selected.folder;
      }

      try {
        await runCtxCommand(["init"], targetFolder.uri.fsPath);
        vscode.window.showInformationMessage("Initialized ctx.toml");

        // Update context
        await vscode.commands.executeCommand("setContext", "ctx.hasCtxToml", true);

        // Open the file
        const ctxTomlUri = vscode.Uri.joinPath(targetFolder.uri, "ctx.toml");
        const doc = await vscode.workspace.openTextDocument(ctxTomlUri);
        await vscode.window.showTextDocument(doc);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        vscode.window.showErrorMessage(`Init failed: ${message}`);
      }
    })
  );
}
