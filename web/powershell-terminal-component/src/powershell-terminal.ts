import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import {
  WasmPowerShellClient,
  WasmWinRmConfig,
} from "../../../crates/ironposh-web/pkg";
import init_wasm, {
  init_tracing_with_level,
  set_panic_hook,
} from "../../../crates/ironposh-web/pkg/ironposh_web";
import { createHostCallHandler } from "./hostcall-handler";
import wasm from "../../../crates/ironposh-web/pkg/ironposh_web_bg.wasm?url";

export interface PowerShellConnectionConfig {
  gateway_url: string;
  gateway_token: string;
  server: string;
  port?: number;
  username: string;
  password: string;
  use_https?: boolean;
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
  private connected: boolean = false;
  private resizeObserver: ResizeObserver | null = null;
  private currentLine: string = "";
  private abortController: AbortController | null = null;

  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.abortController = new AbortController();
  }

  emitEvent(event: PowerShellTerminalEvent) {
    let { type: eventType, detail } = event;

    this.dispatchEvent(new CustomEvent(eventType, { detail }));
  }

  async connectedCallback() {
    // Create container for terminal
    const container = document.createElement("div");
    container.style.width = "100%";
    container.style.height = "100%";
    container.className = "terminal-container";

    // Add xterm.css to shadow DOM
    const style = document.createElement("style");
    style.textContent = `
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

    this.shadowRoot!.appendChild(style);
    this.shadowRoot!.appendChild(container);

    // Initialize terminal
    this.terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Consolas, "Courier New", monospace',
      theme: {
        background: "#012456",
        foreground: "#ffffff",
        cursor: "#ffffff",
      },
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

    this.terminal.writeln(
      `Connecting to ${config.server}:${config.port || 5985}...`
    );

    try {
      const wasmConfig: WasmWinRmConfig = {
        gateway_url: config.gateway_url,
        gateway_token: config.gateway_token,
        server: config.server,
        port: config.port || 5985,
        username: config.username,
        password: config.password,
        use_https: config.use_https || false,
        domain: "",
        locale: "en-US",
      };

      // Create host call handler with terminal integration
      const hostCallHandler = createHostCallHandler({
        terminal: this.terminal,
        hostName: "PowerShell Terminal",
        hostVersion: "1.0.0",
        culture: "en-US",
        uiCulture: "en-US",
      });

      // Create PowerShell client
      this.client = WasmPowerShellClient.connect(wasmConfig, hostCallHandler);

      this.terminal.writeln("✓ Connected successfully!");
      this.terminal.writeln("");
      this.terminal.write("PS> ");
      this.connected = true;

      // Set up terminal input handling
      this.terminal.onData((data) => {
        if (this.connected) {
          this.handleInput(data);
        }
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

    // Handle Enter key
    if (data === "\r") {
      this.terminal.write("\r\n");
      const command = this.currentLine.trim();
      this.currentLine = "";

      if (command) {
        this.executeScript(command, this.abortController!.signal).catch(
          (error) => {
            this.terminal!.writeln(`\x1b[31mError: ${error}\x1b[0m`);
            this.emitEvent({ type: "error", detail: error as Error });
          }
        );
      } else {
        this.terminal.write("PS> ");
      }
    }
    // Handle Backspace
    else if (data === "\u007F" || data === "\b") {
      if (this.currentLine.length > 0) {
        this.currentLine = this.currentLine.slice(0, -1);
        this.terminal.write("\b \b");
      }
    }
    // Handle Ctrl+C
    else if (data === "\u0003") {
      this.terminal.write("^C\r\n");
      this.currentLine = "";
      this.terminal.write("PS> ");
      this.abortController?.abort();
    }
    // Regular character input
    else if (data >= " " || data === "\t") {
      this.currentLine += data;
      this.terminal.write(data);
    }
  }

  async executeScript(script: string, abort: AbortSignal): Promise<void> {
    if (!this.connected || !this.client || !this.terminal) {
      throw new Error("Not connected");
    }

    try {
      let pipeline_id_holder: string | null = null;
      // Execute script (not command - run_script for interactive terminal)
      const stream = await this.client.execute_command(script);
      // Process stream events
      while (true) {
        const event = await stream.next();
        if (abort.aborted) {
          this.terminal.writeln("^C");
          await stream.kill();
        }

        if (!event) {
          break;
        }

        if ("PipelineCreated" in event) {
          pipeline_id_holder = event.PipelineCreated.pipeline_id;
        }

        if ("PipelineFinished" in event) {
          break;
        }

        if ("PipelineOutput" in event) {
          let output = event.PipelineOutput.data;
          this.terminal.write(output);
        }
      }
    } catch (error) {
      this.terminal.writeln(`\x1b[31mError: ${error}\x1b[0m`);
      this.emitEvent({ type: "error", detail: error as Error });
    } finally {
      this.terminal.write("PS> ");
    }
  }

  disconnect(): void {
    if (this.connected) {
      this.connected = false;
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
      if (this.connected) {
        this.terminal.write("PS> ");
      }
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
