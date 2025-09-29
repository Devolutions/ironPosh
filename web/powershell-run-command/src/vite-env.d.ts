/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PWSH_USERNAME?: string;
  readonly VITE_PWSH_PASSWORD?: string;
  readonly VITE_PWSH_DOMAIN?: string;
  readonly VITE_PWSH_HOSTNAME?: string;
  readonly VITE_PWSH_PORT?: string;
  readonly VITE_PWSH_GATEWAY?: string;
  readonly VITE_GATEWAY_WEBAPP_USERNAME?: string;
  readonly VITE_GATEWAY_WEBAPP_PASSWORD?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}