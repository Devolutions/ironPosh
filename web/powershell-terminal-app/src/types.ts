export interface PowerShellConnectionConfig {
  gateway_url: string;
  gateway_token: string;
  server: string;
  port?: number;
  username: string;
  password: string;
  use_https?: boolean;
}

export interface PowerShellTerminalElement extends HTMLElement {
  connect(config: PowerShellConnectionConfig): Promise<void>;
  disconnect(): void;
  clear(): void;
  fit(): void;
  executeCommand(command: string): Promise<string>;
}
