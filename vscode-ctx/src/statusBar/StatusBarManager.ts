import * as vscode from "vscode";
import { ServerLifecycleManager, ServerState } from "../server/lifecycle";

export class StatusBarManager implements vscode.Disposable {
  private statusBarItem: vscode.StatusBarItem;
  private disposables: vscode.Disposable[] = [];

  constructor(private serverManager: ServerLifecycleManager) {
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );

    this.statusBarItem.command = "ctx.showStatusMenu";
    this.updateStatus(serverManager.getState());
    this.statusBarItem.show();

    // Listen for server state changes
    this.disposables.push(
      serverManager.onStateChange((state) => this.updateStatus(state))
    );
  }

  private updateStatus(state: ServerState): void {
    switch (state.status) {
      case "running":
        this.statusBarItem.text = "$(check) ctx";
        this.statusBarItem.tooltip = `ctx server running on port ${state.port}`;
        this.statusBarItem.backgroundColor = undefined;
        break;
      case "starting":
        this.statusBarItem.text = "$(sync~spin) ctx";
        this.statusBarItem.tooltip = "ctx server starting...";
        this.statusBarItem.backgroundColor = undefined;
        break;
      case "stopped":
        this.statusBarItem.text = "$(circle-slash) ctx";
        this.statusBarItem.tooltip = "ctx server stopped. Click to start.";
        this.statusBarItem.backgroundColor = undefined;
        break;
      case "error":
        this.statusBarItem.text = "$(error) ctx";
        this.statusBarItem.tooltip = `ctx server error: ${state.lastError}`;
        this.statusBarItem.backgroundColor = new vscode.ThemeColor(
          "statusBarItem.errorBackground"
        );
        break;
    }
  }

  dispose(): void {
    this.statusBarItem.dispose();
    this.disposables.forEach((d) => d.dispose());
  }
}

export function registerStatusBarCommands(
  context: vscode.ExtensionContext,
  serverManager: ServerLifecycleManager
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.showStatusMenu", async () => {
      const state = serverManager.getState();
      const items: vscode.QuickPickItem[] = [];

      if (state.status === "running") {
        items.push(
          {
            label: "$(debug-stop) Stop Server",
            description: "Stop the ctx MCP server",
          },
          {
            label: "$(refresh) Restart Server",
            description: "Restart the ctx MCP server",
          },
          {
            label: "$(link-external) Open in Browser",
            description: `http://127.0.0.1:${state.port}`,
          },
          {
            label: "$(output) Show Output",
            description: "Show server output log",
          }
        );
      } else if (state.status === "stopped" || state.status === "error") {
        items.push({
          label: "$(play) Start Server",
          description: "Start the ctx MCP server",
        });
        if (state.lastError) {
          items.push({
            label: "$(output) Show Output",
            description: "Show server output log",
          });
        }
      } else if (state.status === "starting") {
        items.push({
          label: "$(output) Show Output",
          description: "Show server output log",
        });
      }

      items.push({
        label: "$(gear) Settings",
        description: "Open ctx extension settings",
      });

      const selected = await vscode.window.showQuickPick(items, {
        placeHolder: `ctx server: ${state.status}`,
      });

      if (!selected) {
        return;
      }

      if (selected.label.includes("Stop")) {
        await serverManager.stopServer();
      } else if (selected.label.includes("Start")) {
        await serverManager.startServer();
      } else if (selected.label.includes("Restart")) {
        await serverManager.stopServer();
        await serverManager.startServer();
      } else if (selected.label.includes("Browser")) {
        vscode.env.openExternal(
          vscode.Uri.parse(`http://127.0.0.1:${state.port}`)
        );
      } else if (selected.label.includes("Output")) {
        serverManager.showOutput();
      } else if (selected.label.includes("Settings")) {
        vscode.commands.executeCommand("workbench.action.openSettings", "ctx");
      }
    })
  );
}
