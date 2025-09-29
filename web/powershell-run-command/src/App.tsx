import React, { useState, useEffect } from 'react';
import init, * as ironposh from 'ironposh-web';
import ConfigForm, { type ConnectionConfig } from './ConfigForm';
import { generateAppToken, generateSessionToken, processToken, uuidv4 } from './tokenService';

const App: React.FC = () => {
  // WASM initialization
  const [wasmReady, setWasmReady] = useState(false);

  // Connection config
  const [config, setConfig] = useState<ConnectionConfig>({
    server: import.meta.env.VITE_PWSH_HOSTNAME || '',
    port: Number(import.meta.env.VITE_PWSH_PORT) || 5985,
    use_https: false,
    username: import.meta.env.VITE_PWSH_USERNAME || '',
    password: import.meta.env.VITE_PWSH_PASSWORD || '',
    domain: import.meta.env.VITE_PWSH_DOMAIN || undefined,
    locale: undefined,
    gateway_url: import.meta.env.VITE_PWSH_GATEWAY || 'http://localhost:7272',
    gateway_token: '',
  });
  const [connected, setConnected] = useState(false);
  const [client, setClient] = useState<any>(null);

  // Initialize WASM on component mount
  useEffect(() => {
    init().then(() => {
      setWasmReady(true);
      ironposh.set_panic_hook();
    }).catch(err => {
      console.error('Failed to initialize WASM:', err);
      setOutput(`Failed to initialize WASM: ${err}`);
    });
  }, []);

  // Command execution
  const [command, setCommand] = useState('');
  const [output, setOutput] = useState('');

  const connect = async () => {
    if (!wasmReady) {
      setOutput('WASM is still initializing, please wait...');
      return;
    }

    if (!config.server.trim() || !config.username.trim() || !config.password.trim()) {
      setOutput('Please fill in all required connection details (server, username, password)');
      return;
    }

    setOutput('Connecting...');

    try {
      // Generate tokens
      const sessionId = uuidv4();
      const protocolStr = 'winrm-http-pwsh';
      const sessionTokenParameters = {
        content_type: 'ASSOCIATION',
        protocol: protocolStr,
        destination: `tcp://${config.server}:${config.port}`,
        lifetime: 60,
        session_id: sessionId,
      };

      const webappUsername = import.meta.env.VITE_GATEWAY_WEBAPP_USERNAME || '';
      const webappPassword = import.meta.env.VITE_GATEWAY_WEBAPP_PASSWORD || '';

      setOutput('Generating gateway tokens...');
      const appToken = await generateAppToken(config.gateway_url, webappUsername, webappPassword);
      const sessionToken = await generateSessionToken(config.gateway_url, sessionTokenParameters, appToken);
      const gatewayUrlWithToken = processToken(config.gateway_url, sessionToken, sessionId);

      setOutput('Connecting to PowerShell...');
      const newClient = ironposh.WasmPowerShellClient.connect({
        server: config.server,
        port: config.port,
        use_https: config.use_https,
        username: config.username,
        password: config.password,
        domain: config.domain,
        locale: config.locale,
        gateway_url: gatewayUrlWithToken,
        gateway_token: sessionToken,
      });
      setClient(newClient);
      setConnected(true);
      setOutput(`Connected to ${config.server} as ${config.username}`);
    } catch (error) {
      setOutput(`Connection error: ${error}`);
      console.error('Connection error:', error);
    }
  };

  const disconnect = () => {
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
      // TODO: Implement actual command execution using ironposh-web client
      setOutput(`Command: ${command}\n\nOutput will appear here once command execution is integrated...`);
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
      <div style={{
        fontFamily: 'monospace',
        maxWidth: '800px',
        margin: '50px auto',
        padding: '20px',
        textAlign: 'center'
      }}>
        <h1>PowerShell Command Runner</h1>
        <p>Initializing WASM module...</p>
      </div>
    );
  }

  return (
    <div style={{
      fontFamily: 'monospace',
      maxWidth: '800px',
      margin: '50px auto',
      padding: '20px'
    }}>
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
          marginBottom: '10px'
        }}
      />
      <button
        onClick={runCommand}
        disabled={!connected}
        style={{
          padding: '10px 20px',
          cursor: connected ? 'pointer' : 'not-allowed',
          opacity: connected ? 1 : 0.5
        }}
      >
        Run Command
      </button>

      {/* Output */}
      <div style={{
        marginTop: '20px',
        padding: '10px',
        border: '1px solid #ccc',
        minHeight: '100px',
        whiteSpace: 'pre-wrap',
        backgroundColor: '#f5f5f5'
      }}>
        {output}
      </div>
    </div>
  );
};

export default App;