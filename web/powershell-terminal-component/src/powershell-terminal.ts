import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import xtermCss from "@xterm/xterm/css/xterm.css?inline";
import {
  JsSessionEvent,
  WasmPowerShellClient,
  WasmWinRmConfig,
} from "../../../crates/ironposh-web/pkg";
import init_wasm, {
  init_tracing_with_level,
  set_panic_hook,
} from "../../../crates/ironposh-web/pkg/ironposh_web";
import { createHostCallHandler } from "./hostcall-handler";
import wasm from "../../../crates/ironposh-web/pkg/ironposh_web_bg.wasm?url";

export type GatewayTransport = "Tcp" | "Tls";

export interface WinRmDestination {
  host: string;
  port: number;
  transport: GatewayTransport;
}

export interface PowerShellConnectionConfig {
  gateway_url: string;
  gateway_token: string;
  destination: WinRmDestination;
  username: string;
  password: string;
  domain?: string;
  force_insecure?: boolean;
  kdc_proxy_url?: string;
  client_computer_name?: string;
}

export interface PowerShellTerminalConfig {
  theme?: {
    background?: string;
    foreground?: string;
    cursor?: string;
  };
  fontSize?: number;
  fontFamily?: string;
  logLevel?: "Error" | "Warn" | "Info" | "Debug" | "Trace";
}

export type PowerShellTerminalEvent =
  | {
      type: "ready";
      detail: undefined;
    }
  | {
      type: "connected";
      detail: undefined;
    }
  | {
      type: "disconnected";
      detail: undefined;
    }
  | {
      type: "error";
      detail: Error;
    };

export class PowerShellTerminalElement extends HTMLElement {
  private terminal: Terminal | null = null;
  private fitAddon: FitAddon | null = null;
  private client: WasmPowerShellClient | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private currentLine: string = "";
  private state: "idle" | "connecting" | "connected" | "closed" = "idle";
  private runningController: AbortController | null = null;
  private isRunning = false;
  private hostCallInputDepth = 0;
  private isPrompting = false;

  constructor() {
    super();
    this.attachShadow({ mode: "open" });
  }

  private setState(newState: typeof this.state) {
    this.state = newState;
    this.setAttribute("state", newState);
  }

  emitEvent(event: PowerShellTerminalEvent) {
    let { type: eventType, detail } = event;

    this.dispatchEvent(new CustomEvent(eventType, { detail }));
  }

  async connectedCallback() {
    // Create container for terminal
    const container = document.createElement("div");
    container.className = "terminal-container";

    // Inject xterm.css into shadow DOM
    const xtermStyle = document.createElement("style");
    xtermStyle.textContent = xtermCss;

    // Add minimal custom styles
    const customStyle = document.createElement("style");
    customStyle.textContent = `
      :host {
        display: block;
        width: 100%;
        height: 100%;
      }
      .terminal-container {
        width: 100%;
        height: 100%;
      }
    `;

    this.shadowRoot!.appendChild(xtermStyle);
    this.shadowRoot!.appendChild(customStyle);
    this.shadowRoot!.appendChild(container);

    // Initialize terminal
    this.terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
    });

    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.loadAddon(new WebLinksAddon());

    this.terminal.open(container);
    this.fitAddon.fit();

    // Set up resize observer
    this.resizeObserver = new ResizeObserver(() => {
      if (this.fitAddon) {
        this.fitAddon.fit();
      }
    });
    this.resizeObserver.observe(container);

    this.terminal.writeln("PowerShell Terminal");
    this.terminal.writeln("Initializing WASM module...");
    this.terminal.writeln("");

    // Load ironposh-web WASM module
    try {
      console.log("Loading WASM from", wasm);
      await init_wasm(wasm);
      set_panic_hook();
      init_tracing_with_level({ Info: undefined } as any);

      this.terminal.writeln("✓ WASM module loaded successfully");
      this.emitEvent({ type: "ready", detail: undefined });
    } catch (error) {
      this.terminal.writeln(`✗ Error loading WASM: ${error}`);
      this.emitEvent({ type: "error", detail: error as Error });
    }
  }

  disconnectedCallback() {
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
    if (this.terminal) {
      this.terminal.dispose();
    }
  }

  async connect(config: PowerShellConnectionConfig): Promise<void> {
    if (!this.terminal) {
      throw new Error("Terminal not initialized");
    }

    this.setState("connecting");

    this.terminal.writeln(
      `Connecting to ${config.destination.host}:${config.destination.port}...`
    );

    try {
      const wasmConfig: WasmWinRmConfig = {
        gateway_url: config.gateway_url,
        gateway_token: config.gateway_token,
        destination: {
          host: config.destination.host,
          port: config.destination.port,
          transport: config.destination.transport,
        },
        username: config.username,
        password: config.password,
        domain: config.domain || undefined,
        locale: "en-US",
        cols: this.terminal.cols,
        rows: this.terminal.rows,
        kdc_proxy_url: config.kdc_proxy_url,
        client_computer_name: config.client_computer_name,
        force_insecure: config.force_insecure,
      };

      console.log("Connecting with config:", { ...config, password: "*****" });

      // Create host call handler with terminal integration
      const hostCallHandler = createHostCallHandler({
        terminal: this.terminal,
        hostName: "PowerShell Terminal",
        hostVersion: "1.0.0",
        culture: "en-US",
        uiCulture: "en-US",
        beginHostCallInput: () => {
          this.hostCallInputDepth += 1;
        },
        endHostCallInput: () => {
          this.hostCallInputDepth = Math.max(0, this.hostCallInputDepth - 1);
        },
      });

      // Create PowerShell client
      await new Promise((resolve, reject) => {
        this.client = WasmPowerShellClient.connect(
          wasmConfig,
          hostCallHandler,
          (event) => {
            if (event === "ActiveSessionStarted") {
              this.setState("connected");
              resolve(undefined);
            }

            if (typeof event === "object" && "error" in event) {
              this.setState("closed");
              reject(new Error(event.error));
            }
          }
        );
      });

      this.setState("connected");

      this.terminal.writeln("✓ Connected successfully!");
      this.terminal.writeln("");
      await this.writePrompt();

      // Set up terminal input handling
      this.terminal.onData((data) => {
        if (this.state !== "connected") return;
        this.handleInput(data);
      });

      this.emitEvent({ type: "connected", detail: undefined });
    } catch (error) {
      this.terminal.writeln(`✗ Connection failed: ${error}`);
      this.emitEvent({ type: "error", detail: error as Error });
      throw error;
    }
  }

  private handleInput(data: string): void {
    if (!this.terminal) return;

    // When a hostcall is actively requesting user input (Read-Host, credential prompts, etc.),
    // do not treat keystrokes as command-line input. Let the hostcall handler consume it.
    if (this.hostCallInputDepth > 0) {
      // Still allow Ctrl+C to cancel a running command.
      if (data === "\u0003") {
        if (this.isRunning && this.runningController) {
          this.terminal.write("^C\r\n");
          this.runningController.abort(new Error("Canceled by user"));
        } else {
          this.currentLine = "";
          this.terminal.write("^C\r\nPS> ");
        }
      }
      return;
    }

    // Ctrl+C
    if (data === "\u0003") {
      if (this.isRunning && this.runningController) {
        this.terminal.write("^C\r\n");
        this.runningController.abort(new Error("Canceled by user"));
        // prompt comes from executeScript.finally
      } else {
        this.currentLine = "";
        this.terminal.write("^C\r\n");
        void this.writePrompt();
      }
      return;
    }

    // Enter
    if (data === "\r") {
      this.terminal.write("\r\n");
      const command = this.currentLine.trim();
      this.currentLine = "";

      if (!command) {
        void this.writePrompt();
        return;
      }

      if (this.isRunning) {
        this.terminal.writeln(
          "(a command is already running — press Ctrl+C to cancel)"
        );
        void this.writePrompt();
        return;
      }

      const controller = new AbortController();
      this.runningController = controller;
      this.isRunning = true;

      this.executeScript(command, controller.signal)
        .catch((err) => {
          this.terminal!.writeln(`\x1b[31mError: ${err}\x1b[0m`);
          this.emitEvent({ type: "error", detail: err as Error });
        })
        .finally(() => {
          this.isRunning = false;
          this.runningController = null;
          this.terminal!.writeln("");
          void this.writePrompt();
        });

      return;
    }

    // Backspace
    if (data === "\u007F" || data === "\b") {
      if (this.currentLine.length > 0) {
        this.currentLine = this.currentLine.slice(0, -1);
        this.terminal.write("\b \b");
      }
      return;
    }

    // Regular printable / tab
    if (data >= " " || data === "\t") {
      this.currentLine += data;
      this.terminal.write(data);
    }
  }

  async executeScript(script: string, signal: AbortSignal): Promise<void> {
    if (this.state !== "connected" || !this.client || !this.terminal) {
      throw new Error("Not connected");
    }

    let created = false;
    let killRequested = false;
    const stream = await this.client.execute_command(script);

    while (true) {
      if (signal.aborted && !killRequested && created) {
        killRequested = true;
        stream.kill();
      }

      const event = await stream.next();
      if (!event) break;

      if ("PipelineCreated" in event) {
        created = true;
        if (killRequested) void stream.kill();
        continue;
      }
      if ("PipelineOutput" in event) {
        this.terminal.writeln(event.PipelineOutput.data);
        continue;
      }
      if ("PipelineError" in event) {
        const errRecord = event.PipelineError.error;
        const errMessage = errRecord.normal_formated_message || errRecord.fully_qualified_error_id || "Unknown error";
        this.terminal.writeln(`\x1b[31mError: ${errMessage}\x1b[0m`);
        this.emitEvent({ type: "error", detail: new Error(errMessage) });
        continue;
      }
      if ("PipelineFinished" in event) break;
    }
  }

  disconnect(): void {
    if (this.state === "connected" && this.terminal) {
      this.setState("closed");
      if (this.terminal) {
        this.terminal.writeln("");
        this.terminal.writeln("Disconnected");
      }
      if (this.client) {
        this.client.disconnect();
        this.client = null;
      }
      this.emitEvent({ type: "disconnected", detail: undefined });
    }
  }

  clear(): void {
    if (this.terminal) {
      this.terminal.clear();
      if (this.state === "connected") {
        void this.writePrompt();
      }
    }
  }

  private async writePrompt(): Promise<void> {
    if (!this.terminal) return;
    if (this.state !== "connected" || !this.client) {
      this.terminal.write("PS> ");
      return;
    }

    if (this.isPrompting) {
      this.terminal.write("PS> ");
      return;
    }

    this.isPrompting = true;
    try {
      // Ask remote PowerShell to render the `prompt` function, to match the tokio
      // interactive experience (path, nested prompt level, custom prompt, etc.).
      //
      // Important: customized prompts may use `Write-Host` and return an empty
      // string. In that case, the prompt has already been rendered via HostCalls,
      // and we should not print any extra local prompt.
      const stream = await this.client.execute_command(
        "prompt",
      );

      const parts: string[] = [];
      while (true) {
        const event = await stream.next();
        if (!event) break;

        if ("PipelineOutput" in event) {
          parts.push(event.PipelineOutput.data);
          continue;
        }

        if ("PipelineError" in event) {
          // Don't surface prompt errors as terminal errors; fallback to static prompt.
          break;
        }

        if ("PipelineFinished" in event) break;
      }

      let prompt = parts.join("");
      // Keep only last line (some prompts include newlines)
      if (prompt.includes("\n") || prompt.includes("\r")) {
        const lines = prompt.split(/\r?\n/).filter((l) => l.length > 0);
        prompt = lines.length > 0 ? lines[lines.length - 1] : "";
      }

      prompt = prompt.replace(/\r?\n$/, "");
      if (prompt.trim().length === 0) {
        // Nothing returned: assume the prompt was rendered via HostCalls (Write-Host).
        return;
      }

      if (!prompt.endsWith(" ")) prompt += " ";
      this.terminal.write(prompt);
    } catch (_e) {
      this.terminal.write("PS> ");
    } finally {
      this.isPrompting = false;
    }
  }

  fit(): void {
    if (this.fitAddon) {
      this.fitAddon.fit();
    }
  }
}

// Register the custom element
customElements.define("powershell-terminal", PowerShellTerminalElement);
