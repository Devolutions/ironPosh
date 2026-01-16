interface TokenCache {
  token: string;
  expiresAt: number;
}

let appTokenCache: TokenCache | null = null;

export async function generateAppToken(
  gatewayUrl: string,
  username?: string,
  password?: string,
  forceRefresh = false
): Promise<string> {
  // Check if we have a valid cached token
  if (!forceRefresh && appTokenCache && appTokenCache.expiresAt > Date.now()) {
    return appTokenCache.token;
  }

  const appTokenApiUrl = '/jet/webapp/app-token';
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Accept: 'application/text',
  };

  if (username && password) {
    headers.Authorization = `Basic ${btoa(`${username}:${password}`)}`;
  }

  const body = {
    content_type: 'WEBAPP',
    subject: username || '',
    lifetime: 7200, // 2 hours
  };

  const response = await fetch(gatewayUrl + appTokenApiUrl, {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    throw new Error(`Failed to generate app token: ${response.statusText}`);
  }

  const token = await response.text();

  // Cache the token with expiration time (slightly less than the lifetime to be safe)
  appTokenCache = {
    token,
    expiresAt: Date.now() + body.lifetime * 1000 * 0.9, // 90% of the lifetime
  };

  return token;
}

export async function generateSessionToken(
  gatewayUrl: string,
  tokenParameters: {
    content_type: string;
    protocol: string;
    destination: string;
    lifetime: number;
    session_id: string;
  },
  appToken: string
): Promise<string> {
  const sessionTokenApiURL = '/jet/webapp/session-token';
  const headers = {
    Authorization: `Bearer ${appToken}`,
    'Content-Type': 'application/json',
  };

  const response = await fetch(gatewayUrl + sessionTokenApiURL, {
    method: 'POST',
    headers,
    body: JSON.stringify(tokenParameters),
  });

  if (!response.ok) {
    throw new Error(`Failed to generate session token: ${response.statusText}`);
  }

  return response.text();
}

export type GatewayTransport = 'Tcp' | 'Tls';

export function processToken(
  gatewayAddress: string,
  token: string,
  sessionId: string,
  transport: GatewayTransport = 'Tcp'
): string {
  const fwdPath = transport === 'Tls' ? 'tls' : 'tcp';
  return `${gatewayAddress}/jet/fwd/${fwdPath}/${sessionId}?token=${token}`;
}

export function getProtocolForTransport(transport: GatewayTransport): string {
  return transport === 'Tls' ? 'winrm-https-pwsh' : 'winrm-http-pwsh';
}

export function getDestinationScheme(transport: GatewayTransport): string {
  return transport === 'Tls' ? 'tls' : 'tcp';
}

export type SecurityWarning = 'GatewayChannelInsecure' | 'DestinationChannelInsecure' | 'BothChannelsInsecure';

export function checkSecurity(
  gatewayUrl: string,
  transport: GatewayTransport,
  forceInsecure: boolean = false
): SecurityWarning[] {
  const gatewaySecure = gatewayUrl.startsWith('wss://') || gatewayUrl.startsWith('https://');
  const sspiEnabled = transport === 'Tcp' && !forceInsecure;
  const destinationSecure = transport === 'Tls' || sspiEnabled;

  // SSPI is end-to-end encryption - if enabled, data is always secure regardless of gateway
  if (sspiEnabled) {
    return []; // End-to-end SSPI encryption - always secure
  }

  // No SSPI - check both channels
  if (gatewaySecure && destinationSecure) {
    return []; // WSS + TLS
  } else if (!gatewaySecure && !destinationSecure) {
    return ['BothChannelsInsecure']; // WS + TCP without SSPI
  } else if (!gatewaySecure) {
    return ['GatewayChannelInsecure']; // WS + TLS (gateway exposed)
  } else {
    return ['DestinationChannelInsecure']; // WSS + TCP without SSPI (destination exposed)
  }
}

export function uuidv4(): string {
  return (String(1e7) + String(-1e3) + String(-4e3) + String(-8e3) + String(-1e11)).replace(/[018]/g, (c: any) =>
    (c ^ (crypto.getRandomValues(new Uint8Array(1))[0] & (15 >> (c / 4)))).toString(16)
  );
}
