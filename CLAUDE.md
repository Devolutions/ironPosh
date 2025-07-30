# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IronWinRM is a Rust implementation for Windows Remote Management (WinRM) and PowerShell Remoting protocols. The project is structured as a Cargo workspace with multiple crates that handle different aspects of remote Windows management.

## Architecture

The project follows a layered architecture with clear separation of concerns:

### Core Crates
- **xml**: Custom XML builder forked from einfach-xml-builder-rs for efficient XML generation
- **protocol-macros**: Procedural macros for protocol code generation
- **protocol-winrm**: Core WinRM protocol implementation with SOAP envelope handling
- **protocol-powershell-remoting**: PowerShell remoting protocol message serialization/deserialization
- **pwsh-core**: High-level PowerShell connection and runspace management

### Protocol Layers
1. **XML Layer** (`crates/xml`): Low-level XML building and parsing
2. **WinRM Protocol Layer** (`crates/protocol-winrm`): SOAP envelopes, WS-Addressing, WS-Management headers
3. **PowerShell Remoting Layer** (`crates/protocol-powershell-remoting`): PowerShell-specific message handling
4. **Core Layer** (`crates/pwsh-core`): Connection management, authentication, runspace operations

### Key Components
- **SOAP Envelope Building**: Complex XML structures for WS-Management communications
- **Authentication**: Basic auth support with extensible credential handling
- **Message Serialization**: PowerShell remoting message format handling
- **Connection Management**: HTTP-based WinRM endpoint communication

## Common Development Commands

### Building
```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p protocol-winrm

# Check for compilation errors without building
cargo check
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p protocol-winrm

# Run specific test
cargo test test_initial_build_request
```

### Examples
```bash
# Run connection example
cargo run --example connect -p pwsh-core

# Run SOAP building example  
cargo run --example build -p protocol-winrm

# Run deserialization example
cargo run --example deserde -p protocol-winrm
```

## Development Patterns

### Error Handling
Each crate defines its own error types using `thiserror`:
- `PwshCoreError` for high-level connection errors
- `PowerShellRemotingError` for protocol-specific errors  
- `ProtocolError` for WinRM protocol errors

### Builder Pattern
Extensively uses `typed-builder` crate for safe construction of complex types like SOAP envelopes, connectors, and configuration objects.

### XML Generation
Custom XML builder with namespace support and typed tag names. Uses compile-time type safety for XML structure validation.

### Tracing
Uses `tracing` crate throughout for structured logging. Examples show proper subscriber initialization for debugging protocol interactions.

## Testing Strategy

Tests are organized by protocol layer:
- Unit tests in `src/` subdirectories
- Integration tests in `tests/` directories  
- Examples serve as integration tests and documentation

Key test files:
- `test_initial_build_request.rs`: SOAP envelope construction
- `test_initial_deserialize_request.rs`: Message deserialization
- Various test modules in `protocol-powershell-remoting/src/tests/`