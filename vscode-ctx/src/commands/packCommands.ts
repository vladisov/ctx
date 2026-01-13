import * as vscode from "vscode";
import { CtxApiClient } from "../api/client";
import { PacksTreeProvider, PackTreeItem } from "../views/PacksTreeProvider";
import { PreviewPanel } from "../views/PreviewPanel";
import { getConfig } from "../config";

export function registerPackCommands(
  context: vscode.ExtensionContext,
  treeProvider: PacksTreeProvider
): void {
  const api = new CtxApiClient();

  // Create Pack
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.createPack", async () => {
      const name = await vscode.window.showInputBox({
        prompt: "Enter pack name",
        placeHolder: "my-feature-pack",
        validateInput: (value) => {
          if (!value || value.trim().length === 0) {
            return "Pack name is required";
          }
          if (!/^[a-zA-Z0-9_:-]+$/.test(value)) {
            return "Pack name can only contain letters, numbers, hyphens, underscores, and colons";
          }
          return null;
        },
      });

      if (!name) {
        return;
      }

      const config = getConfig();
      const defaultBudget = config.get<number>("defaultTokenBudget", 128000);

      const budgetStr = await vscode.window.showInputBox({
        prompt: "Token budget",
        value: defaultBudget.toString(),
        validateInput: (value) => {
          const num = parseInt(value, 10);
          if (isNaN(num) || num <= 0) {
            return "Must be a positive number";
          }
          return null;
        },
      });

      if (!budgetStr) {
        return;
      }

      try {
        const result = await api.createPack({
          name: name.trim(),
          budget_tokens: parseInt(budgetStr, 10),
        });
        vscode.window.showInformationMessage(`Created pack: ${result.name}`);
        treeProvider.refresh();
      } catch (err: unknown) {
        const error = err as { status?: number; error?: string };
        if (error.status === 409) {
          vscode.window.showErrorMessage(`Pack '${name}' already exists`);
        } else {
          vscode.window.showErrorMessage(
            `Failed to create pack: ${error.error || err}`
          );
        }
      }
    })
  );

  // Delete Pack
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.deletePack", async (item?: PackTreeItem) => {
      let packName: string | undefined;

      if (item instanceof PackTreeItem) {
        packName = item.pack.name;
      } else {
        const packs = await api.listPacks();
        const selected = await vscode.window.showQuickPick(
          packs.map((p) => ({ label: p.name, pack: p })),
          { placeHolder: "Select pack to delete" }
        );
        packName = selected?.pack.name;
      }

      if (!packName) {
        return;
      }

      const confirm = await vscode.window.showWarningMessage(
        `Delete pack '${packName}'? This cannot be undone.`,
        { modal: true },
        "Delete"
      );

      if (confirm !== "Delete") {
        return;
      }

      try {
        await api.deletePack(packName);
        vscode.window.showInformationMessage(`Deleted pack: ${packName}`);
        treeProvider.refresh();
      } catch (err: unknown) {
        const error = err as { error?: string };
        vscode.window.showErrorMessage(
          `Failed to delete pack: ${error.error || err}`
        );
      }
    })
  );

  // Preview Pack
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.previewPack", async (item?: PackTreeItem) => {
      let packName: string | undefined;

      if (item instanceof PackTreeItem) {
        packName = item.pack.name;
      } else {
        const packs = await api.listPacks();
        const selected = await vscode.window.showQuickPick(
          packs.map((p) => ({ label: p.name, pack: p })),
          { placeHolder: "Select pack to preview" }
        );
        packName = selected?.pack.name;
      }

      if (!packName) {
        return;
      }

      try {
        const result = await api.renderPack(packName);
        PreviewPanel.createOrShow(packName, result);
      } catch (err: unknown) {
        const error = err as { error?: string };
        vscode.window.showErrorMessage(
          `Failed to render pack: ${error.error || err}`
        );
      }
    })
  );

  // Refresh Packs
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.refreshPacks", () => {
      treeProvider.refresh();
    })
  );

  // Copy Pack Content
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.copyPackContent", async (item?: PackTreeItem) => {
      let packName: string | undefined;

      if (item instanceof PackTreeItem) {
        packName = item.pack.name;
      }

      if (!packName) {
        return;
      }

      try {
        const result = await api.renderPack(packName);
        await vscode.env.clipboard.writeText(result.content);
        vscode.window.showInformationMessage(
          `Copied ${result.token_estimate.toLocaleString()} tokens to clipboard`
        );
      } catch (err: unknown) {
        const error = err as { error?: string };
        vscode.window.showErrorMessage(
          `Failed to render pack: ${error.error || err}`
        );
      }
    })
  );
}
