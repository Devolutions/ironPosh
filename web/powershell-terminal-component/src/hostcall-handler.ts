import type { Terminal } from "@xterm/xterm";
import type { JsHostCall } from "../../../crates/ironposh-web/pkg/ironposh_web";

export interface HostCallHandlerConfig {
  terminal: Terminal;
  hostName?: string;
  hostVersion?: string;
  instanceId?: string;
  culture?: string;
  uiCulture?: string;
}

/**
 * Creates a PowerShell host call handler that integrates with xterm.js terminal
 * This follows the "let it crash" philosophy - only implement what we know how to handle
 */
export function createHostCallHandler(config: HostCallHandlerConfig) {
  const {
    terminal,
    hostName = "IronPoshHost",
    hostVersion = "1.0.0",
    instanceId = crypto.randomUUID(),
    culture = "en-US",
    uiCulture = "en-US",
  } = config;

  return async (hostCall: JsHostCall): Promise<any> => {
    // ===== IMPLEMENTED HOST CALLS (matching ironposh-client-tokio/src/hostcall.rs) =====

    // Basic host information
    if ("GetName" in hostCall) {
      return hostName;
    }

    if ("GetVersion" in hostCall) {
      return hostVersion;
    }

    if ("GetInstanceId" in hostCall) {
      return instanceId;
    }

    if ("GetCurrentCulture" in hostCall) {
      return culture;
    }

    if ("GetCurrentUICulture" in hostCall) {
      return uiCulture;
    }

    // Terminal operations (implemented in Rust version)
    if ("SetCursorPosition" in hostCall) {
      const [x, y] = hostCall.SetCursorPosition.params;
      // For web terminal, we let xterm.js handle cursor positioning naturally
      return;
    }

    if ("SetBufferContents1" in hostCall) {
      // Check if this is a clear screen operation
      const contents = hostCall.SetBufferContents1.params;
      // Simple clear screen detection (would need better parsing for full implementation)
      terminal.clear();
      return;
    }

    if ("WriteProgress" in hostCall) {
      // Progress records - for now just ignore them
      return;
    }

    // ===== UNIMPLEMENTED - THROW ERRORS =====

    const methodName = Object.keys(hostCall)[0];
    throw new Error(
      `Unimplemented host call: ${methodName}. If you need this functionality, implement it in hostcall-handler.ts`
    );
  };
}
