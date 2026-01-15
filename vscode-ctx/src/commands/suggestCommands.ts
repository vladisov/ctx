import * as vscode from "vscode";
import * as path from "path";
import { CtxApiClient } from "../api/client";
import { PacksTreeProvider } from "../views/PacksTreeProvider";

export function registerSuggestCommands(
  context: vscode.ExtensionContext,
  treeProvider: PacksTreeProvider
): void {
  const api = new CtxApiClient();

  // Show suggestions for current file
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.showSuggestions", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage("No file open");
        return;
      }

      const filePath = editor.document.uri.fsPath;

      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "Finding related files...",
          cancellable: false,
        },
        async () => {
          try {
            const response = await api.getSuggestions({
              file: filePath,
              max_results: 15,
            });

            if (response.suggestions.length === 0) {
              vscode.window.showInformationMessage("No suggestions found");
              return;
            }

            // Show Quick Pick with suggestions
            const items = response.suggestions.map((s) => ({
              label: path.basename(s.path),
              description: `${(s.score * 100).toFixed(0)}% match`,
              detail: s.reasons.map((r) => `${r.signal}: ${(r.contribution * 100).toFixed(0)}%`).join(" | "),
              suggestion: s,
            }));

            const selected = await vscode.window.showQuickPick(items, {
              placeHolder: "Select files to add to pack",
              canPickMany: true,
            });

            if (!selected || selected.length === 0) {
              return;
            }

            // Ask which pack to add to
            const packs = await api.listPacks();
            if (packs.length === 0) {
              const create = await vscode.window.showInformationMessage(
                "No packs found. Create one first?",
                "Create Pack"
              );
              if (create === "Create Pack") {
                await vscode.commands.executeCommand("ctx.createPack");
              }
              return;
            }

            const packPick = await vscode.window.showQuickPick(
              packs.map((p) => ({
                label: p.name,
                description: `${p.policies.budget_tokens.toLocaleString()} tokens`,
                pack: p,
              })),
              { placeHolder: "Select pack to add files to" }
            );

            if (!packPick) {
              return;
            }

            // Add selected files to pack
            let addedCount = 0;
            for (const item of selected) {
              try {
                await api.addArtifact(packPick.pack.name, {
                  type: "file",
                  path: item.suggestion.path,
                });
                addedCount++;
              } catch (err) {
                console.error(`Failed to add ${item.suggestion.path}:`, err);
              }
            }

            vscode.window.showInformationMessage(
              `Added ${addedCount} file(s) to ${packPick.pack.name}`
            );
            treeProvider.refresh();
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            vscode.window.showErrorMessage(`Failed to get suggestions: ${message}`);
          }
        }
      );
    })
  );
}
