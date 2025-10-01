# PowerShell Terminal App

A React application that demonstrates using the `powershell-terminal-component` web component.

## Features

- React-based UI for PowerShell remote connections
- Connection management (connect, disconnect, clear)
- Form validation and state management
- Clean, modern interface

## Installation

```bash
npm install
```

## Development

Run the development server:

```bash
npm run dev
```

Open http://localhost:3000

## Building

Build for production:

```bash
npm run build
```

Preview the production build:

```bash
npm run preview
```

## Usage

1. Enter connection details:
   - Server IP address
   - Port (default: 5985)
   - Username
   - Password
   - Enable HTTPS if needed

2. Click "Connect" to establish a PowerShell session

3. Use the terminal to execute PowerShell commands

4. Click "Disconnect" to close the session

5. Click "Clear" to clear the terminal output

## Project Structure

```
powershell-terminal-app/
├── src/
│   ├── App.tsx              # Main application component
│   ├── App.css              # Application styles
│   ├── main.tsx             # Entry point
│   ├── index.css            # Global styles
│   └── types.ts             # TypeScript type definitions
├── index.html
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Integration with Web Component

This app imports the `powershell-terminal-component` as a local dependency and uses it as a custom element in React:

```tsx
import 'powershell-terminal-component/dist/powershell-terminal.js';

function App() {
  const terminalRef = useRef<any>(null);

  return (
    <powershell-terminal ref={terminalRef}></powershell-terminal>
  );
}
```

The component communicates via:
- **Methods**: `connect()`, `disconnect()`, `clear()`, etc.
- **Events**: `ready`, `connected`, `disconnected`, `error`
