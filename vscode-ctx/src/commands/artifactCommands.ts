import * as vscode from "vscode";
import * as path from "path";
import { CtxApiClient } from "../api/client";
import { PacksTreeProvider, ArtifactTreeItem } from "../views/PacksTreeProvider";
import { AddArtifactRequest } from "../api/types";

export function registerArtifactCommands(
  context: vscode.ExtensionContext,
  treeProvider: PacksTreeProvider
): void {
  const api = new CtxApiClient();

  // Helper: Select pack from quick pick
  async function selectPack(): Promise<string | undefined> {
    try {
      const packs = await api.listPacks();
      if (packs.length === 0) {
        const create = await vscode.window.showInformationMessage(
          "No packs found. Create one first?",
          "Create Pack"
        );
        if (create === "Create Pack") {
          await vscode.commands.executeCommand("ctx.createPack");
        }
        return undefined;
      }

      const selected = await vscode.window.showQuickPick(
        packs.map((p) => ({
          label: p.name,
          description: `${p.policies.budget_tokens.toLocaleString()} tokens`,
          pack: p,
        })),
        { placeHolder: "Select pack to add artifact to" }
      );

      return selected?.pack.name;
    } catch (err) {
      vscode.window.showErrorMessage(`Failed to list packs: ${err}`);
      return undefined;
    }
  }

  // Add File from Explorer or Editor
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.addFileToPack", async (uri?: vscode.Uri) => {
      const fileUri = uri || vscode.window.activeTextEditor?.document.uri;

      if (!fileUri || fileUri.scheme !== "file") {
        vscode.window.showErrorMessage("No file selected");
        return;
      }

      const packName = await selectPack();
      if (!packName) {
        return;
      }

      try {
        const artifact: AddArtifactRequest = {
          type: "file",
          path: fileUri.fsPath,
        };

        await api.addArtifact(packName, artifact);
        vscode.window.showInformationMessage(
          `Added ${path.basename(fileUri.fsPath)} to ${packName}`
        );
        treeProvider.refresh();
      } catch (err: unknown) {
        const error = err as { error?: string };
        vscode.window.showErrorMessage(`Failed to add file: ${error.error || err}`);
      }
    })
  );

  // Add Selection as Text
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.addSelectionToPack", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.selection.isEmpty) {
        vscode.window.showErrorMessage("No text selected");
        return;
      }

      const selectedText = editor.document.getText(editor.selection);
      const packName = await selectPack();
      if (!packName) {
        return;
      }

      try {
        const artifact: AddArtifactRequest = {
          type: "text",
          content: selectedText,
        };

        await api.addArtifact(packName, artifact);
        vscode.window.showInformationMessage(`Added selection to ${packName}`);
        treeProvider.refresh();
      } catch (err: unknown) {
        const error = err as { error?: string };
        vscode.window.showErrorMessage(
          `Failed to add selection: ${error.error || err}`
        );
      }
    })
  );

  // Remove Artifact
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.removeArtifact", async (item?: ArtifactTreeItem) => {
      if (!(item instanceof ArtifactTreeItem)) {
        return;
      }

      const confirm = await vscode.window.showWarningMessage(
        `Remove artifact from pack '${item.packName}'?`,
        { modal: true },
        "Remove"
      );

      if (confirm !== "Remove") {
        return;
      }

      try {
        await api.removeArtifact(item.packName, item.artifact.id);
        vscode.window.showInformationMessage("Artifact removed");
        treeProvider.refresh();
      } catch (err: unknown) {
        const error = err as { error?: string };
        vscode.window.showErrorMessage(
          `Failed to remove artifact: ${error.error || err}`
        );
      }
    })
  );
}
