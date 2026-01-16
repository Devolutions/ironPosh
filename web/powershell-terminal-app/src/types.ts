import type { GatewayTransport } from 'gateway-token-service';

export type { GatewayTransport } from 'gateway-token-service';

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
}

export interface PowerShellTerminalElement extends HTMLElement {
  connect(config: PowerShellConnectionConfig): Promise<void>;
  disconnect(): void;
  clear(): void;
  fit(): void;
  executeCommand(command: string): Promise<string>;
}
