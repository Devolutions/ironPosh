import React, { useState, useEffect } from 'react';
import init, * as ironposh from 'ironposh-web';
import ConfigForm, { type ConnectionConfig } from './ConfigForm';
import {
  generateAppToken,
  generateSessionToken,
  processToken,
  getProtocolForTransport,
  uuidv4,
} from './tokenService';

// Security warning type from WASM
type SecurityWarning =
  | 'GatewayChannelInsecure'
  | 'DestinationChannelInsecure'
  | 'BothChannelsInsecure';

const App: React.FC = () => {
  // WASM initialization
  const [wasmReady, setWasmReady] = useState(false);

  // Connection config
  const [config, setConfig] = useState<ConnectionConfig>({
    destination: {
      host: import.meta.env.VITE_PWSH_HOSTNAME || '',
      port: Number(import.meta.env.VITE_PWSH_PORT) || 5985,
      transport: 'Tcp', // Default to TCP with SSPI sealing
    },
    username: import.meta.env.VITE_PWSH_USERNAME || '',
    password: import.meta.env.VITE_PWSH_PASSWORD || '',
    domain: import.meta.env.VITE_PWSH_DOMAIN || undefined,
    locale: undefined,
    gateway_url: import.meta.env.VITE_PWSH_GATEWAY || 'http://localhost:7272',
    gateway_token: '',
    force_insecure: false,
  });
  const [connected, setConnected] = useState(false);
  const [client, setClient] = useState<ironposh.WasmPowerShellClient | null>(null);

  // Initialize WASM on component mount
  useEffect(() => {
    init()
      .then(() => {
        setWasmReady(true);
        ironposh.set_panic_hook();
        ironposh.init_tracing_with_level('Debug');
      })
      .catch((err) => {
        console.error('Failed to initialize WASM:', err);
        setOutput(`Failed to initialize WASM: ${err}`);
      });
  }, []);

  // Command execution
  const [command, setCommand] = useState('');
  const [output, setOutput] = useState('');

  // Security warning handler
  const handleSecurityWarning = async (warnings: SecurityWarning[]): Promise<boolean> => {
    const warningMessages = warnings.map((w) => {
      switch (w) {
        case 'GatewayChannelInsecure':
          return '- Gateway channel is using WS instead of WSS (unencrypted)';
        case 'DestinationChannelInsecure':
          return '- Destination channel has no encryption (HTTP without SSPI)';
        case 'BothChannelsInsecure':
          return '- BOTH channels are unencrypted! This is extremely dangerous!';
        default:
          return `- Unknown warning: ${w}`;
      }
    });

    const message = `Security Warnings Detected:\n\n${warningMessages.join('\n')}\n\nDo you want to continue with this insecure connection?`;

    return window.confirm(message);
  };

  const connect = async () => {
    if (!wasmReady) {
      setOutput('WASM is still initializing, please wait...');
      return;
    }

    if (
      !config.destination.host.trim() ||
      !config.username.trim() ||
      !config.password.trim()
    ) {
      setOutput('Please fill in all required connection details (host, username, password)');
      return;
    }

    setOutput('Connecting...');

    try {
      // Generate tokens
      const sessionId = uuidv4();
      const protocolStr = getProtocolForTransport(config.destination.transport);
      const destinationScheme = config.destination.transport === 'Tls' ? 'tls' : 'tcp';
      const sessionTokenParameters = {
        content_type: 'ASSOCIATION',
        protocol: protocolStr,
        destination: `${destinationScheme}://${config.destination.host}:${config.destination.port}`,
        lifetime: 60,
        session_id: sessionId,
      };

      const webappUsername = import.meta.env.VITE_GATEWAY_WEBAPP_USERNAME || '';
      const webappPassword = import.meta.env.VITE_GATEWAY_WEBAPP_PASSWORD || '';

      setOutput('Generating gateway tokens...');
      const appToken = await generateAppToken(config.gateway_url, webappUsername, webappPassword);
      const sessionToken = await generateSessionToken(
        config.gateway_url,
        sessionTokenParameters,
        appToken
      );
      const gatewayUrlWithToken = processToken(
        config.gateway_url,
        sessionToken,
        sessionId,
        config.destination.transport
      );

      setOutput('Connecting to PowerShell...');

      // Build WASM config
      const wasmConfig = {
        destination: {
          host: config.destination.host,
          port: config.destination.port,
          transport: config.destination.transport,
        },
        gateway_url: gatewayUrlWithToken,
        gateway_token: sessionToken,
        username: config.username,
        password: config.password,
        domain: config.domain,
        locale: config.locale,
        kdc_proxy_url: undefined,
        client_computer_name: undefined,
        cols: 120,
        rows: 30,
        force_insecure: config.force_insecure,
      };

      // Host call handler
      const hostCallHandler = (hostCall: any) => {
        console.log('Host call received:', hostCall);
        // Return null for now - can be extended to handle interactive prompts
        return null;
      };

      // Session event handler
      const sessionEventHandler = (event: any) => {
        console.log('Session event:', event);
        if (event === 'Closed' || event?.error) {
          setConnected(false);
          setClient(null);
          setOutput((prev) => prev + '\nSession closed.');
        }
      };

      // Connect with security check
      const newClient = await ironposh.WasmPowerShellClient.connect_with_security_check(
        wasmConfig,
        hostCallHandler,
        sessionEventHandler,
        handleSecurityWarning
      );

      setClient(newClient);
      setConnected(true);
      setOutput(`Connected to ${config.destination.host} as ${config.username}`);
    } catch (error) {
      setOutput(`Connection error: ${error}`);
      console.error('Connection error:', error);
    }
  };

  const disconnect = () => {
    if (client) {
      client.disconnect();
    }
    setClient(null);
    setConnected(false);
    setOutput('Disconnected');
  };

  const runCommand = async () => {
    if (!connected || !client) {
      setOutput('Please connect first');
      return;
    }

    if (!command.trim()) {
      setOutput('Please enter a command');
      return;
    }

    setOutput('Running command...');

    try {
      const stream = await client.execute_command(command);

      while (true) {
        const event = await stream.next();
        console.log('Received event:', event);
        if (!event) break;
        if ('PipelineOutput' in event) {
          setOutput((prev) => prev + event.PipelineOutput.data + '\n');
        } else if ('PipelineError' in event) {
          setOutput((prev) => prev + `ERROR: ${event.PipelineError.error}\n`);
        } else if ('PipelineFinished' in event) {
          setOutput((prev) => prev + '\nCommand execution finished.\n');
          break;
        }
      }

      stream.free();
    } catch (error) {
      setOutput(`Error: ${error}`);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      runCommand();
    }
  };

  if (!wasmReady) {
    return (
      <div
        style={{
          fontFamily: 'monospace',
          maxWidth: '800px',
          margin: '50px auto',
          padding: '20px',
          textAlign: 'center',
        }}
      >
        <h1>PowerShell Command Runner</h1>
        <p>Initializing WASM module...</p>
      </div>
    );
  }

  return (
    <div
      style={{
        fontFamily: 'monospace',
        maxWidth: '800px',
        margin: '50px auto',
        padding: '20px',
      }}
    >
      <h1>PowerShell Command Runner</h1>

      {/* Connection Configuration */}
      <ConfigForm
        config={config}
        onChange={setConfig}
        onConnect={connect}
        onDisconnect={disconnect}
        connected={connected}
      />

      {/* Command Execution */}
      <h2>Run Command</h2>
      <input
        type="text"
        value={command}
        onChange={(e) => setCommand(e.target.value)}
        onKeyPress={handleKeyPress}
        placeholder="Enter PowerShell command..."
        disabled={!connected}
        style={{
          width: '100%',
          padding: '10px',
          fontFamily: 'monospace',
          marginBottom: '10px',
        }}
      />
      <button
        onClick={runCommand}
        disabled={!connected}
        style={{
          padding: '10px 20px',
          cursor: connected ? 'pointer' : 'not-allowed',
          opacity: connected ? 1 : 0.5,
        }}
      >
        Run Command
      </button>

      {/* Output */}
      <div
        style={{
          marginTop: '20px',
          padding: '10px',
          border: '1px solid #ccc',
          minHeight: '100px',
          whiteSpace: 'pre-wrap',
          backgroundColor: '#f5f5f5',
        }}
      >
        {output}
      </div>
    </div>
  );
};

export default App;
