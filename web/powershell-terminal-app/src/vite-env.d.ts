/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PWSH_TER_GATEWAY_URL: string
  readonly VITE_PWSH_TER_GATEWAY_WEBAPP_USERNAME: string
  readonly VITE_PWSH_TER_GATEWAY_WEBAPP_PASSWORD: string
  readonly VITE_PWSH_TER_SERVER: string
  readonly VITE_PWSH_TER_PORT: string
  readonly VITE_PWSH_TER_USERNAME: string
  readonly VITE_PWSH_TER_PASSWORD: string
  readonly VITE_PWSH_TER_USE_HTTPS: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
