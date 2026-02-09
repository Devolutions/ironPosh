import React, { useEffect, useMemo, useState } from 'react';
import init, * as ironposh from 'ironposh-web';
import type {
  HostCallHandlers,
  HostCallTag,
  JsHostCall,
  JsPsValue,
  JsRunCommandEvent,
  TypedHostCallHandler,
} from 'ironposh-web';
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

type TableRow = Record<string, string>;

const formatPrimitive = (primitive: { kind: string; value?: unknown }): string => {
  switch (primitive.kind) {
    case 'str':
      return String(primitive.value ?? '');
    case 'bool':
    case 'i32':
    case 'u32':
    case 'i64':
    case 'u64':
      return String(primitive.value ?? '');
    case 'guid':
    case 'version':
    case 'dateTime':
    case 'timeSpan':
      return String(primitive.value ?? '');
    case 'char':
      return String(primitive.value ?? '');
    case 'bytes':
      return `[bytes ${Array.isArray(primitive.value) ? primitive.value.length : 0}]`;
    case 'secureString':
      return `[secure ${Array.isArray(primitive.value) ? primitive.value.length : 0}]`;
    case 'nil':
      return 'null';
    default:
      return `[unknown ${primitive.kind}]`;
  }
};

const formatPsValue = (value: JsPsValue | null | undefined): string => {
  if (!value) return '';
  if (value.kind === 'primitive') {
    return formatPrimitive(value.value as { kind: string; value?: unknown });
  }
  if (value.kind === 'object') {
    const obj = value.value as {
      toString?: string | null;
      adaptedProperties?: Record<string, { name: string; value: JsPsValue }> | Map<string, { name: string; value: JsPsValue }>;
      extendedProperties?: Record<string, { name: string; value: JsPsValue }> | Map<string, { name: string; value: JsPsValue }>;
      content?: { kind: string; value?: unknown };
    };
    if (obj.toString) return obj.toString;
    const keys: string[] = [];
    const addKeys = (
      props?: Record<string, { name: string; value: JsPsValue }> | Map<string, { name: string; value: JsPsValue }>
    ) => {
      if (!props) return;
      if (props instanceof Map) {
        props.forEach((_v, k) => keys.push(k));
        return;
      }
      Object.keys(props).forEach((k) => keys.push(k));
    };
    addKeys(obj.adaptedProperties);
    addKeys(obj.extendedProperties);
    if (keys.length > 0) return `[object ${keys.join(', ')}]`;
    if (obj.content?.kind === 'container') return '[container]';
    return '[object]';
  }
  return '[unknown]';
};

const psValueToTableRow = (value: JsPsValue): TableRow | null => {
  if (!value || value.kind !== 'object') return null;
  const obj = value.value as {
    adaptedProperties?: Record<string, { name: string; value: JsPsValue }> | Map<string, { name: string; value: JsPsValue }>;
    extendedProperties?: Record<string, { name: string; value: JsPsValue }> | Map<string, { name: string; value: JsPsValue }>;
  };
  const row: TableRow = {};
  const addProps = (
    props?: Record<string, { name: string; value: JsPsValue }> | Map<string, { name: string; value: JsPsValue }>
  ) => {
    if (!props) return;
    if (props instanceof Map) {
      props.forEach((prop, key) => {
        row[key] = formatPsValue(prop.value);
      });
      return;
    }
    Object.keys(props).forEach((key) => {
      row[key] = formatPsValue(props[key]?.value);
    });
  };
  addProps(obj.adaptedProperties);
  addProps(obj.extendedProperties);
  if (Object.keys(row).length === 0) return null;
  return row;
};

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
        setStatus(`Failed to initialize WASM: ${err}`);
      });
  }, []);

  // Command execution
  const [command, setCommand] = useState('');
  const [status, setStatus] = useState('');
  const [eventLog, setEventLog] = useState<string[]>([]);
  const [rawValues, setRawValues] = useState<string[]>([]);
  const [tableRows, setTableRows] = useState<TableRow[]>([]);
  const [tableColumns, setTableColumns] = useState<string[]>([]);

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
      setStatus('WASM is still initializing, please wait...');
      return;
    }

    if (
      !config.destination.host.trim() ||
      !config.username.trim() ||
      !config.password.trim()
    ) {
      setStatus('Please fill in all required connection details (host, username, password)');
      return;
    }

    setStatus('Connecting...');

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

      setStatus('Generating gateway tokens...');
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

      setStatus('Connecting to PowerShell...');

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

      // Host call handler - minimal stub for non-interactive command execution
      // Uses HostCallHandlers for type-safe handler definitions
      const stubHandlers = {
        // Host methods (1-10)
        GetName: () => 'PowerShellRunCommand',
        GetVersion: () => '1.0.0',
        GetInstanceId: () => crypto.randomUUID(),
        GetCurrentCulture: () => 'en-US',
        GetCurrentUICulture: () => 'en-US',
        SetShouldExit: () => {},
        EnterNestedPrompt: () => {},
        ExitNestedPrompt: () => {},
        NotifyBeginApplication: () => {},
        NotifyEndApplication: () => {},

        // Input methods - throw since this app doesn't support interactive input
        ReadLine: () => { throw new Error('Interactive ReadLine not supported'); },
        ReadLineAsSecureString: () => { throw new Error('Interactive ReadLineAsSecureString not supported'); },

        // Output methods - log to console
        Write1: (text) => { console.log('[PS Write]', text); },
        Write2: ([_fg, _bg, text]) => { console.log('[PS Write]', text); },
        WriteLine1: () => { console.log('[PS WriteLine]'); },
        WriteLine2: (text) => { console.log('[PS WriteLine]', text); },
        WriteLine3: ([_fg, _bg, text]) => { console.log('[PS WriteLine]', text); },
        WriteErrorLine: (text) => { console.error('[PS Error]', text); },
        WriteDebugLine: (text) => { console.debug('[PS Debug]', text); },
        WriteProgress: ({ record }) => { console.log('[PS Progress]', record.activity, record.statusDescription); },
        WriteVerboseLine: (text) => { console.log('[PS Verbose]', text); },
        WriteWarningLine: (text) => { console.warn('[PS Warning]', text); },
        Prompt: () => { throw new Error('Interactive Prompt not supported'); },
        PromptForCredential1: () => { throw new Error('Interactive PromptForCredential not supported'); },
        PromptForCredential2: () => { throw new Error('Interactive PromptForCredential not supported'); },
        PromptForChoice: () => { throw new Error('Interactive PromptForChoice not supported'); },

        // RawUI methods - return reasonable defaults
        GetForegroundColor: () => 7,
        SetForegroundColor: () => {},
        GetBackgroundColor: () => 0,
        SetBackgroundColor: () => {},
        GetCursorPosition: () => ({ x: 0, y: 0 }),
        SetCursorPosition: () => {},
        GetWindowPosition: () => ({ x: 0, y: 0 }),
        SetWindowPosition: () => {},
        GetCursorSize: () => 25,
        SetCursorSize: () => {},
        GetBufferSize: () => ({ width: 120, height: 30 }),
        SetBufferSize: () => {},
        GetWindowSize: () => ({ width: 120, height: 30 }),
        SetWindowSize: () => {},
        GetWindowTitle: () => 'PowerShell',
        SetWindowTitle: () => {},
        GetMaxWindowSize: () => ({ width: 120, height: 30 }),
        GetMaxPhysicalWindowSize: () => ({ width: 120, height: 30 }),
        GetKeyAvailable: () => false,
        ReadKey: () => { throw new Error('Interactive ReadKey not supported'); },
        FlushInputBuffer: () => {},
        SetBufferContents1: () => {},
        SetBufferContents2: () => {},
        GetBufferContents: () => [],
        ScrollBufferContents: () => {},

        // Interactive session methods
        PushRunspace: () => {},
        PopRunspace: () => {},
        GetIsRunspacePushed: () => false,
        GetRunspace: () => ({ kind: 'primitive', value: { kind: 'str', value: '' } }),
        PromptForChoiceMultipleSelection: () => { throw new Error('Interactive PromptForChoiceMultipleSelection not supported'); },
      } satisfies HostCallHandlers;

      // Dispatch function to convert JsHostCall to handler call
      const hostCallHandler = ((call: JsHostCall) => {
        const tag = Object.keys(call)[0] as HostCallTag;
        const variant = call[tag as keyof typeof call] as { params: unknown };
        const handler = stubHandlers[tag];
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        return (handler as any)(variant.params);
      }) as TypedHostCallHandler;

      // Session event handler
      const sessionEventHandler = (event: any) => {
        console.log('Session event:', event);
        if (event === 'Closed' || event?.error) {
          setConnected(false);
          setClient(null);
          setStatus((prev) => `${prev}\nSession closed.`);
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
      setStatus(`Connected to ${config.destination.host} as ${config.username}`);
    } catch (error) {
      setStatus(`Connection error: ${error}`);
      console.error('Connection error:', error);
    }
  };

  const disconnect = () => {
    if (client) {
      client.disconnect();
    }
    setClient(null);
    setConnected(false);
    setStatus('Disconnected');
  };

  const runCommand = async () => {
    if (!connected || !client) {
      setStatus('Please connect first');
      return;
    }

    if (!command.trim()) {
      setStatus('Please enter a command');
      return;
    }

    setStatus('Running command...');
    setEventLog([]);
    setRawValues([]);
    setTableRows([]);
    setTableColumns([]);

    try {
      await client.runCommand(command, (event: JsRunCommandEvent) => {
        setEventLog((prev) => [...prev, `${new Date().toLocaleTimeString()} ${event.type}`]);
        if (event.type === 'pipelineOutput') {
          const row = psValueToTableRow(event.value);
          setRawValues((prev) => [...prev, JSON.stringify(event.value, null, 2)]);
          if (row) {
            setTableRows((prev) => [...prev, row]);
            setTableColumns((prev) => {
              const next = [...prev];
              for (const key of Object.keys(row)) {
                if (!next.includes(key)) next.push(key);
              }
              return next;
            });
          }
        } else if (event.type === 'pipelineError') {
          setStatus(`Command error: ${event.error?.message ?? 'unknown error'}`);
        } else if (event.type === 'pipelineFinished') {
          setStatus('Command execution finished.');
        }
      });
    } catch (error) {
      setStatus(`Error: ${error}`);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      runCommand();
    }
  };

  const tableHeader = useMemo(() => {
    if (tableColumns.length === 0) return null;
    return (
      <tr>
        {tableColumns.map((col) => (
          <th key={col} style={{ textAlign: 'left', padding: '6px', borderBottom: '1px solid #ccc' }}>
            {col}
          </th>
        ))}
      </tr>
    );
  }, [tableColumns]);

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

      {/* Status */}
      <div
        style={{
          marginTop: '20px',
          padding: '10px',
          border: '1px solid #ccc',
          minHeight: '50px',
          whiteSpace: 'pre-wrap',
          backgroundColor: '#f5f5f5',
        }}
      >
        {status}
      </div>

      {/* Table Output */}
      <div style={{ marginTop: '20px' }}>
        <h3>Structured Output</h3>
        {tableColumns.length === 0 ? (
          <div style={{ color: '#666' }}>No structured rows yet.</div>
        ) : (
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>{tableHeader}</thead>
            <tbody>
              {tableRows.map((row, idx) => (
                <tr key={idx}>
                  {tableColumns.map((col) => (
                    <td key={`${idx}-${col}`} style={{ padding: '6px', borderBottom: '1px solid #eee' }}>
                      {row[col] ?? ''}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Event Log */}
      <div style={{ marginTop: '20px' }}>
        <h3>Event Log</h3>
        <div
          style={{
            maxHeight: '200px',
            overflow: 'auto',
            border: '1px solid #ddd',
            padding: '10px',
            backgroundColor: '#fafafa',
          }}
        >
          {eventLog.length === 0 ? 'No events yet.' : eventLog.join('\n')}
        </div>
      </div>

      {/* Raw Values */}
      <div style={{ marginTop: '20px' }}>
        <h3>Raw JsPsValue</h3>
        <div
          style={{
            maxHeight: '300px',
            overflow: 'auto',
            border: '1px solid #ddd',
            padding: '10px',
            backgroundColor: '#fafafa',
            whiteSpace: 'pre-wrap',
          }}
        >
          {rawValues.length === 0 ? 'No values yet.' : rawValues.join('\n\n')}
        </div>
      </div>
    </div>
  );
};

export default App;
