# PowerShell Terminal Component

A web component that wraps xterm.js with ironposh-web for PowerShell remoting in the browser.

## Features

- Custom `<powershell-terminal>` web component
- Built on xterm.js for terminal rendering
- Integrates with ironposh-web WASM module for PowerShell remoting
- Framework-agnostic (works with React, Vue, vanilla JS, etc.)

## Installation

```bash
npm install
```

## Building

Build the WASM module and component:

```bash
npm run build
```

This will:
1. Build the ironposh-web WASM module
2. Bundle the web component with Vite

## Development

Run the development server:

```bash
npm run dev
```

Open http://localhost:5173 to see the component demo.

## Usage

### In HTML

```html
<script type="module" src="path/to/powershell-terminal.js"></script>

<powershell-terminal id="terminal"></powershell-terminal>

<script type="module">
  const terminal = document.getElementById('terminal');

  terminal.addEventListener('ready', () => {
    console.log('Terminal ready');
  });

  terminal.addEventListener('connected', () => {
    console.log('Connected');
  });

  // Connect to PowerShell
  await terminal.connect({
    server: '192.168.1.100',
    port: 5985,
    username: 'Administrator',
    password: 'password',
    useHttps: false
  });
</script>
```

### In React

See the `powershell-terminal-app` project for a complete React example.

## API

### Methods

- `connect(config: PowerShellConnectionConfig): Promise<void>` - Connect to a PowerShell remote session
- `disconnect(): void` - Disconnect from the session
- `executeCommand(command: string): Promise<string>` - Execute a PowerShell command
- `clear(): void` - Clear the terminal
- `fit(): void` - Fit the terminal to its container

### Events

- `ready` - Fired when the terminal is initialized and ready
- `connected` - Fired when successfully connected to PowerShell
- `disconnected` - Fired when disconnected
- `error` - Fired when an error occurs (detail contains error object)

## Project Structure

```
powershell-terminal-component/
├── src/
│   └── powershell-terminal.ts    # Main web component
├── wasm-pkg/                      # Generated WASM module (ignored)
├── dist/                          # Built component (ignored)
├── package.json
├── vite.config.js
└── tsconfig.json
```
