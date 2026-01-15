import * as vscode from "vscode";
import { ServerLifecycleManager } from "./server/lifecycle";
import { PacksTreeProvider } from "./views/PacksTreeProvider";
import { CtxTomlWatcher } from "./ctxToml/watcher";
import { StatusBarManager, registerStatusBarCommands } from "./statusBar/StatusBarManager";
import { registerPackCommands } from "./commands/packCommands";
import { registerArtifactCommands } from "./commands/artifactCommands";
import { registerSyncCommands } from "./commands/syncCommands";
import { registerSuggestCommands } from "./commands/suggestCommands";

let serverManager: ServerLifecycleManager;
let statusBarManager: StatusBarManager;
let tomlWatcher: CtxTomlWatcher;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  console.log("ctx extension activating...");

  // Initialize server lifecycle manager
  serverManager = new ServerLifecycleManager();
  context.subscriptions.push(serverManager);

  // Initialize ctx.toml watcher
  tomlWatcher = new CtxTomlWatcher();
  context.subscriptions.push(tomlWatcher);
  await tomlWatcher.checkCtxTomlExists();

  // Initialize tree view
  const packsTreeProvider = new PacksTreeProvider();
  const treeView = vscode.window.createTreeView("ctx.packsView", {
    treeDataProvider: packsTreeProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView);

  // Initialize status bar
  statusBarManager = new StatusBarManager(serverManager);
  context.subscriptions.push(statusBarManager);

  // Register commands
  registerPackCommands(context, packsTreeProvider);
  registerArtifactCommands(context, packsTreeProvider);
  registerSyncCommands(context, packsTreeProvider);
  registerSuggestCommands(context, packsTreeProvider);
  registerStatusBarCommands(context, serverManager);

  // Register server commands
  context.subscriptions.push(
    vscode.commands.registerCommand("ctx.startServer", () =>
      serverManager.startServer()
    ),
    vscode.commands.registerCommand("ctx.stopServer", () =>
      serverManager.stopServer()
    )
  );

  // Try to connect to or start server
  const connected = await serverManager.ensureServerRunning();
  if (connected) {
    packsTreeProvider.refresh();
  } else {
    vscode.window.showWarningMessage(
      'ctx server not running. Start it with "ctx: Start Server" command or check that ctx is installed.'
    );
  }

  // Listen for ctx.toml changes
  tomlWatcher.onCtxTomlChange(() => {
    vscode.window
      .showInformationMessage("ctx.toml changed. Sync packs?", "Sync")
      .then((action) => {
        if (action === "Sync") {
          vscode.commands.executeCommand("ctx.syncFromToml");
        }
      });
  });

  // Refresh packs when server comes online
  serverManager.onStateChange((state) => {
    if (state.status === "running") {
      packsTreeProvider.refresh();
    }
  });

  console.log("ctx extension activated");
}

export function deactivate(): void {
  // Cleanup is handled by disposables
}
