import { useEffect, useRef, useState } from 'react';
import './App.css';
import 'powershell-terminal-component';
import type { PowerShellConnectionConfig, GatewayTransport, WinRmDestination } from './types';
import {
  generateAppToken,
  generateSessionToken,
  processToken,
  uuidv4,
  getProtocolForTransport,
  getDestinationScheme,
  checkSecurity,
  type SecurityWarning,
} from 'gateway-token-service';

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
  destination: WinRmDestination;
  username: string;
  password: string;
  domain: string;
  force_insecure: boolean;
}

function getSecurityStatusColor(gatewayUrl: string, transport: GatewayTransport, forceInsecure: boolean): string {
  const warnings = checkSecurity(gatewayUrl, transport, forceInsecure);
  if (warnings.length === 0) return '#4caf50'; // green
  if (warnings.includes('BothChannelsInsecure')) return '#f44336'; // red
  return '#ff9800'; // orange
}

function getSecurityStatusText(gatewayUrl: string, transport: GatewayTransport, forceInsecure: boolean): string {
  const gatewaySecure = gatewayUrl.startsWith('wss://') || gatewayUrl.startsWith('https://');

  const gatewayStatus = gatewaySecure ? 'Secure (WSS/HTTPS)' : 'Insecure (WS/HTTP)';
  const destStatus = transport === 'Tls' ? 'Secure (TLS)' : (forceInsecure ? 'Insecure (no SSPI)' : 'Secure (SSPI)');

  return `Gateway: ${gatewayStatus} | Destination: ${destStatus}`;
}

function formatSecurityWarnings(warnings: SecurityWarning[]): string {
  return warnings.map(w => {
    switch (w) {
      case 'GatewayChannelInsecure':
        return '⚠️ Gateway channel is not using TLS (WS/HTTP instead of WSS/HTTPS)';
      case 'DestinationChannelInsecure':
        return '⚠️ Destination channel is not encrypted (no TLS and SSPI encryption disabled)';
      case 'BothChannelsInsecure':
        return '🚨 BOTH channels are insecure! Gateway uses WS/HTTP and destination has no encryption';
      default:
        return `⚠️ Unknown warning: ${w}`;
    }
  }).join('\n');
}

function App() {
  const terminalRef = useRef<any>(null);
  const [isReady, setIsReady] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [formData, setFormData] = useState<ConnectionFormData>({
    gateway_url: import.meta.env.VITE_PWSH_TER_GATEWAY_URL || 'http://localhost:7272',
    gateway_webapp_username: import.meta.env.VITE_PWSH_TER_GATEWAY_WEBAPP_USERNAME || '',
    gateway_webapp_password: import.meta.env.VITE_PWSH_TER_GATEWAY_WEBAPP_PASSWORD || '',
    destination: {
      host: import.meta.env.VITE_PWSH_TER_SERVER || '192.168.1.100',
      port: parseInt(import.meta.env.VITE_PWSH_TER_PORT || '5985'),
      transport: (import.meta.env.VITE_PWSH_TER_TRANSPORT as GatewayTransport) || 'Tcp',
    },
    username: import.meta.env.VITE_PWSH_TER_USERNAME || 'Administrator',
    password: import.meta.env.VITE_PWSH_TER_PASSWORD || '',
    domain: import.meta.env.VITE_PWSH_TER_DOMAIN || '',
    force_insecure: false,
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
      // Check security warnings
      const warnings = checkSecurity(formData.gateway_url, formData.destination.transport, formData.force_insecure);
      if (warnings.length > 0) {
        const warningText = formatSecurityWarnings(warnings);
        const shouldContinue = window.confirm(
          `Security Warning:\n\n${warningText}\n\nDo you want to continue with this insecure configuration?`
        );
        if (!shouldContinue) {
          return;
        }
      }

      // Generate tokens
      const sessionId = uuidv4();
      const protocolStr = getProtocolForTransport(formData.destination.transport);
      const destinationScheme = getDestinationScheme(formData.destination.transport);
      const sessionTokenParameters = {
        content_type: 'ASSOCIATION',
        protocol: protocolStr,
        destination: `${destinationScheme}://${formData.destination.host}:${formData.destination.port}`,
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
      const gatewayUrlWithToken = processToken(
        formData.gateway_url,
        sessionToken,
        sessionId,
        formData.destination.transport
      );

      const config: PowerShellConnectionConfig = {
        gateway_url: gatewayUrlWithToken,
        gateway_token: sessionToken,
        destination: formData.destination,
        username: formData.username,
        password: formData.password,
        domain: formData.domain || undefined,
        force_insecure: formData.force_insecure,
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

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value, type } = e.target;
    const checked = (e.target as HTMLInputElement).checked;

    if (name.startsWith('destination.')) {
      const field = name.split('.')[1];
      setFormData(prev => ({
        ...prev,
        destination: {
          ...prev.destination,
          [field]: field === 'port' ? parseInt(value) || 0 : value,
        },
      }));
    } else {
      setFormData(prev => ({
        ...prev,
        [name]: type === 'checkbox' ? checked : value,
      }));
    }
  };

  const securityColor = getSecurityStatusColor(formData.gateway_url, formData.destination.transport, formData.force_insecure);
  const securityText = getSecurityStatusText(formData.gateway_url, formData.destination.transport, formData.force_insecure);

  return (
    <div className="app">
      <header className="app-header">
        <h1>PowerShell Remote Terminal</h1>
      </header>

      <div className="connection-panel">
        {/* Security Status */}
        <div className="security-status" style={{ backgroundColor: securityColor, padding: '8px', borderRadius: '4px', marginBottom: '16px', color: 'white' }}>
          {securityText}
        </div>

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
          <label htmlFor="destination.host">Server:</label>
          <input
            type="text"
            id="destination.host"
            name="destination.host"
            value={formData.destination.host}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="192.168.1.100"
          />
        </div>

        <div className="form-group">
          <label htmlFor="destination.port">Port:</label>
          <input
            type="number"
            id="destination.port"
            name="destination.port"
            value={formData.destination.port}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="5985"
          />
        </div>

        <div className="form-group">
          <label htmlFor="destination.transport">Transport:</label>
          <select
            id="destination.transport"
            name="destination.transport"
            value={formData.destination.transport}
            onChange={handleInputChange}
            disabled={isConnected}
          >
            <option value="Tcp">TCP (SSPI encryption enabled)</option>
            <option value="Tls">TLS (SSPI encryption disabled)</option>
          </select>
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

        <div className="form-group">
          <label htmlFor="domain">Domain:</label>
          <input
            type="text"
            id="domain"
            name="domain"
            value={formData.domain}
            onChange={handleInputChange}
            disabled={isConnected}
            placeholder="DOMAIN (optional)"
          />
        </div>

        <div className="form-group checkbox">
          <label>
            <input
              type="checkbox"
              name="force_insecure"
              checked={formData.force_insecure}
              onChange={handleInputChange}
              disabled={isConnected || formData.destination.transport === 'Tls'}
            />
            Force Insecure (disable SSPI encryption for TCP)
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
