import React from 'react';

// Gateway transport mode - how gateway connects to WinRM
export type GatewayTransport = 'Tcp' | 'Tls';

// WinRM destination configuration
export interface WinRmDestination {
  host: string;
  port: number;
  transport: GatewayTransport;
}

export interface ConnectionConfig {
  destination: WinRmDestination;
  username: string;
  password: string;
  domain?: string;
  locale?: string;
  gateway_url: string;
  gateway_token: string;
  force_insecure?: boolean;
}

interface ConfigFormProps {
  config: ConnectionConfig;
  onChange: (config: ConnectionConfig) => void;
  onConnect: () => void;
  onDisconnect: () => void;
  connected: boolean;
}

const ConfigForm: React.FC<ConfigFormProps> = ({
  config,
  onChange,
  onConnect,
  onDisconnect,
  connected,
}) => {
  const handleChange = <K extends keyof ConnectionConfig>(
    field: K,
    value: ConnectionConfig[K]
  ) => {
    onChange({ ...config, [field]: value });
  };

  const handleDestinationChange = <K extends keyof WinRmDestination>(
    field: K,
    value: WinRmDestination[K]
  ) => {
    onChange({
      ...config,
      destination: { ...config.destination, [field]: value },
    });
  };

  // Check if gateway URL is secure (WSS)
  const isGatewaySecure = config.gateway_url.toLowerCase().startsWith('wss://');

  // Check if destination is secure (TLS or TCP with SSPI)
  const isDestinationSecure =
    config.destination.transport === 'Tls' || !config.force_insecure;

  return (
    <div
      style={{
        padding: '15px',
        border: '1px solid #ddd',
        borderRadius: '5px',
        marginBottom: '20px',
        backgroundColor: '#f9f9f9',
      }}
    >
      <h2>Connection Configuration</h2>

      {/* Security Status */}
      <div
        style={{
          padding: '10px',
          marginBottom: '15px',
          borderRadius: '4px',
          backgroundColor:
            isGatewaySecure && isDestinationSecure ? '#d4edda' : '#fff3cd',
          border: `1px solid ${isGatewaySecure && isDestinationSecure ? '#c3e6cb' : '#ffc107'}`,
        }}
      >
        <strong>Security Status:</strong>
        <ul style={{ margin: '5px 0', paddingLeft: '20px' }}>
          <li>
            Gateway Channel:{' '}
            {isGatewaySecure ? (
              <span style={{ color: 'green' }}>Secure (WSS)</span>
            ) : (
              <span style={{ color: 'orange' }}>Insecure (WS)</span>
            )}
          </li>
          <li>
            Destination Channel:{' '}
            {config.destination.transport === 'Tls' ? (
              <span style={{ color: 'green' }}>Secure (TLS)</span>
            ) : config.force_insecure ? (
              <span style={{ color: 'red' }}>INSECURE (No encryption)</span>
            ) : (
              <span style={{ color: 'green' }}>Secure (SSPI sealing)</span>
            )}
          </li>
        </ul>
      </div>

      {/* Destination Host */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          WinRM Host:
        </label>
        <input
          type="text"
          value={config.destination.host}
          onChange={(e) => handleDestinationChange('host', e.target.value)}
          placeholder="e.g., server.domain.com or 192.168.1.100"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Destination Port */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          WinRM Port:
        </label>
        <input
          type="number"
          value={config.destination.port}
          onChange={(e) =>
            handleDestinationChange('port', parseInt(e.target.value) || 5985)
          }
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Gateway Transport Mode */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Gateway → WinRM Transport:
        </label>
        <select
          value={config.destination.transport}
          onChange={(e) =>
            handleDestinationChange('transport', e.target.value as GatewayTransport)
          }
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        >
          <option value="Tcp">TCP (HTTP, port 5985) - SSPI encrypts messages</option>
          <option value="Tls">TLS (HTTPS, port 5986) - TLS encrypts connection</option>
        </select>
      </div>

      {/* Force Insecure - only show for TCP transport */}
      {config.destination.transport === 'Tcp' && (
        <div style={{ marginBottom: '10px' }}>
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              fontSize: '14px',
              color: config.force_insecure ? 'red' : 'inherit',
            }}
          >
            <input
              type="checkbox"
              checked={config.force_insecure || false}
              onChange={(e) => handleChange('force_insecure', e.target.checked)}
              disabled={connected}
              style={{ marginRight: '8px' }}
            />
            Disable SSPI encryption (DANGEROUS - for testing only)
          </label>
        </div>
      )}

      <hr style={{ margin: '15px 0', border: 'none', borderTop: '1px solid #ddd' }} />

      {/* Username */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Username:
        </label>
        <input
          type="text"
          value={config.username}
          onChange={(e) => handleChange('username', e.target.value)}
          placeholder="Username"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Password */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Password:
        </label>
        <input
          type="password"
          value={config.password}
          onChange={(e) => handleChange('password', e.target.value)}
          placeholder="Password"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Domain (optional) */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Domain (optional):
        </label>
        <input
          type="text"
          value={config.domain || ''}
          onChange={(e) => handleChange('domain', e.target.value || undefined)}
          placeholder="Domain"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Locale (optional) */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Locale (optional):
        </label>
        <input
          type="text"
          value={config.locale || ''}
          onChange={(e) => handleChange('locale', e.target.value || undefined)}
          placeholder="e.g., en-US"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      <hr style={{ margin: '15px 0', border: 'none', borderTop: '1px solid #ddd' }} />

      {/* Gateway URL */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Gateway URL:
        </label>
        <input
          type="text"
          value={config.gateway_url}
          onChange={(e) => handleChange('gateway_url', e.target.value)}
          placeholder="wss://gateway.example.com or ws://localhost:7272"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Gateway Token */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Gateway Token:
        </label>
        <input
          type="text"
          value={config.gateway_token}
          onChange={(e) => handleChange('gateway_token', e.target.value)}
          placeholder="Token (auto-generated on connect)"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* Connect/Disconnect Button */}
      <div style={{ marginTop: '15px' }}>
        {!connected ? (
          <button
            onClick={onConnect}
            style={{
              padding: '10px 20px',
              cursor: 'pointer',
              backgroundColor: '#4CAF50',
              color: 'white',
              border: 'none',
              borderRadius: '3px',
              fontSize: '14px',
            }}
          >
            Connect
          </button>
        ) : (
          <button
            onClick={onDisconnect}
            style={{
              padding: '10px 20px',
              cursor: 'pointer',
              backgroundColor: '#f44336',
              color: 'white',
              border: 'none',
              borderRadius: '3px',
              fontSize: '14px',
            }}
          >
            Disconnect
          </button>
        )}
        <span
          style={{
            marginLeft: '10px',
            color: connected ? 'green' : 'red',
            fontWeight: 'bold',
          }}
        >
          {connected ? '● Connected' : '○ Not connected'}
        </span>
      </div>
    </div>
  );
};

export default ConfigForm;
