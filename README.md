# IronWinRM

A Rust implementation of the WinRM (Windows Remote Management) and PowerShell Remoting Protocol (PSRP) for remote Windows system management.

## Overview

IronWinRM provides a comprehensive set of libraries and clients for communicating with Windows systems via WinRM and executing PowerShell commands remotely. The project is built as a Cargo workspace with multiple specialized crates supporting different use cases including async/sync operations, terminal interfaces, and WebAssembly integration.

## Architecture

The project is organized into several core components:

### Protocol Libraries

- **ironposh-winrm**: Core WinRM protocol implementation including SOAP message handling, WS-Management, and WS-Addressing support
- **ironposh-psrp**: PowerShell Remoting Protocol (PSRP) implementation with message serialization/deserialization and fragmentation support
- **ironposh-xml**: Custom XML builder and parser (forked from einfach-xml-builder-rs)

### Client Libraries

- **ironposh-client-core**: Core client functionality including connection management, authentication, runspace pools, and pipeline operations
- **ironposh-async**: Async client implementation for non-blocking operations
- **ironposh-client-sync**: Synchronous client with blocking operations and Kerberos support
- **ironposh-client-tokio**: Tokio-based async client with interactive REPL and non-interactive command execution modes
- **ironposh-web**: WebAssembly client for browser-based PowerShell remoting

### Support Libraries

- **ironposh-terminal**: Terminal input/output handling and rendering
- **ironposh-macros**: Procedural macros for the project

## Features

- Full WinRM protocol support with SOAP message handling
- PowerShell Remoting Protocol (PSRP) implementation
- Multiple authentication methods (Basic, NTLM, Kerberos)
- Message encryption and secure transport
- Connection pooling and session management
- Async and sync client APIs
- Interactive REPL mode for PowerShell sessions
- WebAssembly support for browser-based clients
- Structured logging with tracing

## Getting Started

### Prerequisites

- Rust 2024 edition or later
- For WebAssembly: wasm-pack and wasm-bindgen
- Target Windows system with WinRM enabled

### Building

Build the entire workspace:

```bash
cargo build
```

Build a specific crate:

```bash
cargo build -p ironposh-client-tokio
```

### Usage Examples

#### Tokio Async Client

Interactive mode:

```bash
cargo run --bin ironposh-client-tokio -- -s 192.168.1.100 -u Administrator -P MyPassword
```

Non-interactive mode:

```bash
cargo run --bin ironposh-client-tokio -- -s 192.168.1.100 -u Administrator -P MyPassword -c "Get-ComputerInfo"
```

Command line options:
- `-s, --server <IP>`: Server IP address
- `-p, --port <PORT>`: Server port (default: 5985)
- `-u, --username <USER>`: Username for authentication
- `-P, --password <PASS>`: Password for authentication
- `--https`: Use HTTPS instead of HTTP
- `-v, --verbose`: Increase logging verbosity
- `-c, --command <CMD>`: Execute command in non-interactive mode

#### Web Client

The web client provides a React-based interface for PowerShell remoting:

```bash
cd web/powershell-run-command
npm install
npm run dev
```

## Development

### Project Structure

```
ironwinrm/
├── crates/
│   ├── ironposh-async/          # Async client implementation
│   ├── ironposh-client-core/    # Core client functionality
│   ├── ironposh-client-sync/    # Synchronous client
│   ├── ironposh-client-tokio/   # Tokio-based client with REPL
│   ├── ironposh-macros/         # Procedural macros
│   ├── ironposh-psrp/           # PSRP protocol implementation
│   ├── ironposh-terminal/       # Terminal I/O handling
│   ├── ironposh-web/            # WebAssembly client
│   ├── ironposh-winrm/          # WinRM protocol implementation
│   └── ironposh-xml/            # XML parser and builder
└── web/
    └── powershell-run-command/  # React web interface
```

### Testing

Run tests for all crates:

```bash
cargo test
```

Run tests for a specific crate:

```bash
cargo test -p ironposh-psrp
```

### Logging

Structured logging is implemented using the `tracing` crate. Enable verbose logging with:

```bash
RUST_LOG=debug cargo run
```

Or use the `-v` flag with supported clients for increased verbosity.

## License

See LICENSE file for details.

## Contributing

Contributions are welcome. Please ensure all tests pass and follow the existing code style.

## Acknowledgments

- XML builder component forked from [xml-builder-rs](https://github.com/Deaths-Door/xml-builder-rs)