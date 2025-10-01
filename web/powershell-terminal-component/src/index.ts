// Re-export all public types and classes
export { PowerShellTerminalElement } from './powershell-terminal';
export type {
  PowerShellConnectionConfig,
  PowerShellTerminalConfig,
  PowerShellTerminalEvent
} from './powershell-terminal';

// Auto-register the custom element on import
import './powershell-terminal';
