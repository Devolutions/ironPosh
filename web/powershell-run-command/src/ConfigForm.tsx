import React from 'react';

export interface ConnectionConfig {
  server: string;
  port: number;
  use_https: boolean;
  username: string;
  password: string;
  domain?: string;
  locale?: string;
  gateway_url: string;
  gateway_token: string;
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
  const handleChange = (field: keyof ConnectionConfig, value: string | number | boolean | undefined) => {
    onChange({ ...config, [field]: value });
  };

  return (
    <div style={{
      padding: '15px',
      border: '1px solid #ddd',
      borderRadius: '5px',
      marginBottom: '20px',
      backgroundColor: '#f9f9f9'
    }}>
      <h2>Connection Configuration</h2>

      {/* Server */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Server:
        </label>
        <input
          type="text"
          value={config.server}
          onChange={(e) => handleChange('server', e.target.value)}
          placeholder="e.g., localhost or 192.168.1.100"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box'
          }}
        />
      </div>

      {/* Port */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Port:
        </label>
        <input
          type="number"
          value={config.port}
          onChange={(e) => handleChange('port', parseInt(e.target.value) || 5985)}
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box'
          }}
        />
      </div>

      {/* Use HTTPS */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'flex', alignItems: 'center', fontSize: '14px' }}>
          <input
            type="checkbox"
            checked={config.use_https}
            onChange={(e) => handleChange('use_https', e.target.checked)}
            disabled={connected}
            style={{ marginRight: '8px' }}
          />
          Use HTTPS
        </label>
      </div>

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
            boxSizing: 'border-box'
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
            boxSizing: 'border-box'
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
            boxSizing: 'border-box'
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
            boxSizing: 'border-box'
          }}
        />
      </div>

      {/* Gateway URL */}
      <div style={{ marginBottom: '10px' }}>
        <label style={{ display: 'block', marginBottom: '3px', fontSize: '14px' }}>
          Gateway URL:
        </label>
        <input
          type="text"
          value={config.gateway_url}
          onChange={(e) => handleChange('gateway_url', e.target.value)}
          placeholder="http://localhost"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box'
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
          placeholder="Token (if required)"
          disabled={connected}
          style={{
            width: '100%',
            padding: '8px',
            fontFamily: 'monospace',
            boxSizing: 'border-box'
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
              fontSize: '14px'
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
              fontSize: '14px'
            }}
          >
            Disconnect
          </button>
        )}
        <span style={{ marginLeft: '10px', color: connected ? 'green' : 'red', fontWeight: 'bold' }}>
          {connected ? '● Connected' : '○ Not connected'}
        </span>
      </div>
    </div>
  );
};

export default ConfigForm;