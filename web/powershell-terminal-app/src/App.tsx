import { useEffect, useRef, useState } from 'react';
import './App.css';
import 'powershell-terminal-component';
import type { PowerShellConnectionConfig } from './types';
import { generateAppToken, generateSessionToken, processToken, uuidv4 } from 'gateway-token-service';

// Declare custom element for TypeScript
declare global {
  namespace JSX {
    interface IntrinsicElements {
      'powershell-terminal': React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
    }
  }
}

interface ConnectionFormData {
  gateway_url: string;
  gateway_webapp_username: string;
  gateway_webapp_password: string;
  server: string;
  port: number;
  username: string;
  password: string;
  use_https: boolean;
}

function App() {
  const terminalRef = useRef<any>(null);
  const [isReady, setIsReady] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [formData, setFormData] = useState<ConnectionFormData>({
    gateway_url: import.meta.env.VITE_PWSH_TER_GATEWAY_URL || 'http://localhost:7272',
    gateway_webapp_username: import.meta.env.VITE_PWSH_TER_GATEWAY_WEBAPP_USERNAME || '',
    gateway_webapp_password: import.meta.env.VITE_PWSH_TER_GATEWAY_WEBAPP_PASSWORD || '',
    server: import.meta.env.VITE_PWSH_TER_SERVER || '192.168.1.100',
    port: parseInt(import.meta.env.VITE_PWSH_TER_PORT || '5985'),
    username: import.meta.env.VITE_PWSH_TER_USERNAME || 'Administrator',
    password: import.meta.env.VITE_PWSH_TER_PASSWORD || '',
    use_https: import.meta.env.VITE_PWSH_TER_USE_HTTPS === 'true'
  });

  useEffect(() => {
    const terminal = terminalRef.current;

    if (terminal) {
      const handleReady = () => {
        console.log('Terminal ready');
        setIsReady(true);
      };

      const handleConnected = () => {
        console.log('Connected to PowerShell');
        setIsConnected(true);
      };

      const handleDisconnected = () => {
        console.log('Disconnected from PowerShell');
        setIsConnected(false);
      };

      const handleError = (e: CustomEvent) => {
        console.error('Terminal error:', e.detail);
      };

      terminal.addEventListener('ready', handleReady);
      terminal.addEventListener('connected', handleConnected);
      terminal.addEventListener('disconnected', handleDisconnected);
      terminal.addEventListener('error', handleError);

      return () => {
        terminal.removeEventListener('ready', handleReady);
        terminal.removeEventListener('connected', handleConnected);
        terminal.removeEventListener('disconnected', handleDisconnected);
        terminal.removeEventListener('error', handleError);
      };
    }
  }, []);

  const handleConnect = async () => {
    if (!terminalRef.current || !isReady) return;

    try {
      // Generate tokens
      const sessionId = uuidv4();
      const protocolStr = 'winrm-http-pwsh';
      const sessionTokenParameters = {
        content_type: 'ASSOCIATION',
        protocol: protocolStr,
        destination: `tcp://${formData.server}:${formData.port}`,
        lifetime: 60,
        session_id: sessionId,
      };

      console.log('Generating gateway tokens...');
      const appToken = await generateAppToken(
        formData.gateway_url,
        formData.gateway_webapp_username,
        formData.gateway_webapp_password
      );
      const sessionToken = await generateSessionToken(formData.gateway_url, sessionTokenParameters, appToken);
      const gatewayUrlWithToken = processToken(formData.gateway_url, sessionToken, sessionId);

      const config: PowerShellConnectionConfig = {
        gateway_url: gatewayUrlWithToken,
        gateway_token: sessionToken,
        server: formData.server,
        port: formData.port,
        username: formData.username,
        password: formData.password,
        use_https: formData.use_https
      };

      await terminalRef.current.connect(config);
    } catch (error) {
      console.error('Connection failed:', error);
      alert(`Connection failed: ${error}`);
    }
  };

  const handleDisconnect = () => {
    if (terminalRef.current) {
      terminalRef.current.disconnect();
    }
  };

  const handleClear = () => {
    if (terminalRef.current) {
      terminalRef.current.clear();
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value, type, checked } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : (name === 'port' ? parseInt(value) || 0 : value)
    }));
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>PowerShell Remote Terminal</h1>
      </header>

      <div className="connection-panel">
        <div className="form-group">
          <label htmlFor="gateway_url">Gateway URL:</label>
          <input
            type="text"
            id="gateway_url"
            name="gateway_url"
            value={formData.gateway_url}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="http://localhost:7272"
          />
        </div>

        <div className="form-group">
          <label htmlFor="gateway_webapp_username">Gateway Webapp Username:</label>
          <input
            type="text"
            id="gateway_webapp_username"
            name="gateway_webapp_username"
            value={formData.gateway_webapp_username}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="webapp-user"
          />
        </div>

        <div className="form-group">
          <label htmlFor="gateway_webapp_password">Gateway Webapp Password:</label>
          <input
            type="password"
            id="gateway_webapp_password"
            name="gateway_webapp_password"
            value={formData.gateway_webapp_password}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="webapp-password"
          />
        </div>

        <div className="form-group">
          <label htmlFor="server">Server:</label>
          <input
            type="text"
            id="server"
            name="server"
            value={formData.server}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="192.168.1.100"
          />
        </div>

        <div className="form-group">
          <label htmlFor="port">Port:</label>
          <input
            type="number"
            id="port"
            name="port"
            value={formData.port}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="5985"
          />
        </div>

        <div className="form-group">
          <label htmlFor="username">Username:</label>
          <input
            type="text"
            id="username"
            name="username"
            value={formData.username}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="Administrator"
          />
        </div>

        <div className="form-group">
          <label htmlFor="password">Password:</label>
          <input
            type="password"
            id="password"
            name="password"
            value={formData.password}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="Password"
          />
        </div>

        <div className="form-group checkbox">
          <label>
            <input
              type="checkbox"
              name="use_https"
              checked={formData.use_https}
              onChange={handleInputChange}
              disabled={isConnected}
            />
            Use HTTPS
          </label>
        </div>

        <div className="button-group">
          <button
            onClick={handleConnect}
            disabled={!isReady || isConnected}
            className="btn-primary"
          >
            Connect
          </button>
          <button
            onClick={handleDisconnect}
            disabled={!isConnected}
            className="btn-secondary"
          >
            Disconnect
          </button>
          <button
            onClick={handleClear}
            disabled={!isReady}
            className="btn-secondary"
          >
            Clear
          </button>
        </div>
      </div>

      <div className="terminal-container">
        <powershell-terminal ref={terminalRef}></powershell-terminal>
      </div>
    </div>
  );
}

export default App;
