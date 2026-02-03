import type { WinRmDestination as IronposhWinRmDestination } from '../../../crates/ironposh-web/pkg/ironposh_web';

export type { GatewayTransport } from 'gateway-token-service';

// Re-export from ironposh-web for consistency
export type WinRmDestination = IronposhWinRmDestination;

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
