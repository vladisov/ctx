import * as vscode from "vscode";
import { RenderResult } from "../api/types";

export class PreviewPanel {
  public static currentPanel: PreviewPanel | undefined;
  private readonly panel: vscode.WebviewPanel;
  private disposables: vscode.Disposable[] = [];

  private constructor(panel: vscode.WebviewPanel) {
    this.panel = panel;
    this.panel.onDidDispose(() => this.dispose(), null, this.disposables);
  }

  public static createOrShow(packName: string, renderResult: RenderResult): void {
    const column = vscode.ViewColumn.Beside;

    if (PreviewPanel.currentPanel) {
      PreviewPanel.currentPanel.panel.reveal(column);
      PreviewPanel.currentPanel.update(packName, renderResult);
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      "ctxPreview",
      `ctx: ${packName}`,
      column,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
      }
    );

    PreviewPanel.currentPanel = new PreviewPanel(panel);
    PreviewPanel.currentPanel.update(packName, renderResult);

    // Handle messages from webview
    panel.webview.onDidReceiveMessage(
      (message) => {
        if (message.type === "copied") {
          vscode.window.showInformationMessage("Content copied to clipboard");
        }
      },
      null,
      PreviewPanel.currentPanel.disposables
    );
  }

  public update(packName: string, result: RenderResult): void {
    this.panel.title = `ctx: ${packName}`;
    this.panel.webview.html = this.getHtmlContent(packName, result);
  }

  private getHtmlContent(packName: string, result: RenderResult): string {
    const escapedContent = this.escapeHtml(result.content);

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ctx Preview: ${packName}</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            padding: 20px;
            color: var(--vscode-foreground);
            background-color: var(--vscode-editor-background);
        }
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 20px;
            padding-bottom: 10px;
            border-bottom: 1px solid var(--vscode-panel-border);
        }
        .stats {
            display: flex;
            gap: 20px;
        }
        .stat {
            text-align: center;
        }
        .stat-value {
            font-size: 24px;
            font-weight: bold;
            color: var(--vscode-textLink-foreground);
        }
        .stat-label {
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
        }
        .content {
            background-color: var(--vscode-textCodeBlock-background);
            padding: 15px;
            border-radius: 4px;
            white-space: pre-wrap;
            font-family: var(--vscode-editor-font-family);
            font-size: var(--vscode-editor-font-size);
            overflow-x: auto;
            max-height: calc(100vh - 150px);
            overflow-y: auto;
        }
        .copy-button {
            background-color: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
            border: none;
            padding: 8px 16px;
            border-radius: 4px;
            cursor: pointer;
        }
        .copy-button:hover {
            background-color: var(--vscode-button-hoverBackground);
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>${this.escapeHtml(packName)}</h1>
        <div class="stats">
            <div class="stat">
                <div class="stat-value">${result.token_estimate.toLocaleString()}</div>
                <div class="stat-label">tokens</div>
            </div>
        </div>
        <button class="copy-button" onclick="copyContent()">Copy to Clipboard</button>
    </div>
    <pre class="content" id="content">${escapedContent}</pre>
    <script>
        const vscode = acquireVsCodeApi();
        function copyContent() {
            const content = document.getElementById('content').textContent;
            navigator.clipboard.writeText(content).then(() => {
                vscode.postMessage({ type: 'copied' });
            });
        }
    </script>
</body>
</html>`;
  }

  private escapeHtml(text: string): string {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  public dispose(): void {
    PreviewPanel.currentPanel = undefined;
    this.panel.dispose();
    while (this.disposables.length) {
      const disposable = this.disposables.pop();
      if (disposable) {
        disposable.dispose();
      }
    }
  }
}
