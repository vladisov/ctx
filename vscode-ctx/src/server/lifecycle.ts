import * as vscode from "vscode";
import * as cp from "child_process";
import { getConfig } from "../config";
import { checkServerHealth } from "./health";

export type ServerStatus = "stopped" | "starting" | "running" | "error";

export interface ServerState {
  status: ServerStatus;
  port: number;
  lastError?: string;
}

export class ServerLifecycleManager implements vscode.Disposable {
  private state: ServerState = { status: "stopped", port: 17373 };
  private process?: cp.ChildProcess;
  private healthCheckInterval?: NodeJS.Timeout;
  private outputChannel: vscode.OutputChannel;
  private _onStateChange = new vscode.EventEmitter<ServerState>();

  readonly onStateChange = this._onStateChange.event;

  constructor() {
    this.outputChannel = vscode.window.createOutputChannel("ctx Server");
  }

  async ensureServerRunning(): Promise<boolean> {
    const config = getConfig();
    const port = config.get<number>("server.port", 17373);
    const host = config.get<string>("server.host", "127.0.0.1");

    // Check if already running
    if (await checkServerHealth(host, port)) {
      this.updateState({ status: "running", port });
      this.startHealthCheck();
      return true;
    }

    // Auto-start if configured
    if (config.get<boolean>("server.autoStart", true)) {
      return this.startServer();
    }

    return false;
  }

  async startServer(): Promise<boolean> {
    const config = getConfig();
    const ctxPath = config.get<string>("ctxBinaryPath") || "ctx";
    const port = config.get<number>("server.port", 17373);
    const host = config.get<string>("server.host", "127.0.0.1");

    this.updateState({ status: "starting", port });
    this.outputChannel.appendLine(`Starting ctx server on ${host}:${port}...`);

    try {
      this.process = cp.spawn(
        ctxPath,
        ["mcp", "--host", host, "--port", port.toString()],
        {
          detached: true,
          stdio: ["ignore", "pipe", "pipe"],
        }
      );

      this.process.stdout?.on("data", (data) => {
        this.outputChannel.appendLine(data.toString().trim());
      });

      this.process.stderr?.on("data", (data) => {
        this.outputChannel.appendLine(`[stderr] ${data.toString().trim()}`);
      });

      this.process.on("error", (err) => {
        this.outputChannel.appendLine(`Server error: ${err.message}`);
        this.updateState({ status: "error", port, lastError: err.message });
      });

      this.process.on("exit", (code) => {
        this.outputChannel.appendLine(`Server exited with code ${code}`);
        if (this.state.status === "running") {
          this.updateState({ status: "stopped", port });
        }
      });

      // Wait for server to be ready
      const ready = await this.waitForReady(host, port, 10000);
      if (ready) {
        this.outputChannel.appendLine("Server started successfully");
        this.updateState({ status: "running", port });
        this.startHealthCheck();
        return true;
      } else {
        this.process.kill();
        this.updateState({
          status: "error",
          port,
          lastError: "Server failed to start",
        });
        return false;
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      this.outputChannel.appendLine(`Failed to start server: ${message}`);
      this.updateState({ status: "error", port, lastError: message });
      return false;
    }
  }

  async stopServer(): Promise<void> {
    this.stopHealthCheck();
    if (this.process) {
      this.process.kill();
      this.process = undefined;
    }
    this.updateState({ status: "stopped", port: this.state.port });
    this.outputChannel.appendLine("Server stopped");
  }

  private async waitForReady(
    host: string,
    port: number,
    timeout: number
  ): Promise<boolean> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      if (await checkServerHealth(host, port)) {
        return true;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    return false;
  }

  private startHealthCheck(): void {
    const config = getConfig();
    const host = config.get<string>("server.host", "127.0.0.1");

    this.healthCheckInterval = setInterval(async () => {
      const healthy = await checkServerHealth(host, this.state.port);
      if (!healthy && this.state.status === "running") {
        this.updateState({
          ...this.state,
          status: "error",
          lastError: "Connection lost",
        });
      } else if (healthy && this.state.status === "error") {
        this.updateState({ ...this.state, status: "running", lastError: undefined });
      }
    }, 30000);
  }

  private stopHealthCheck(): void {
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
      this.healthCheckInterval = undefined;
    }
  }

  private updateState(newState: ServerState): void {
    this.state = newState;
    this._onStateChange.fire(this.state);
  }

  getState(): ServerState {
    return this.state;
  }

  showOutput(): void {
    this.outputChannel.show();
  }

  dispose(): void {
    this.stopHealthCheck();
    if (this.process) {
      this.process.kill();
    }
    this.outputChannel.dispose();
    this._onStateChange.dispose();
  }
}
