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

## Core XML Processing Traits

### TagValue Trait (`crates/protocol-winrm/src/cores/tag_value.rs:10`)

The `TagValue` trait is the core abstraction for XML element content generation during serialization:

```rust
pub trait TagValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a>;
}
```

**Key Implementations:**
- `Text<'a>`: Text content with `Cow<'a, str>` for efficient string handling
- `Empty`: Empty XML elements (self-closing tags)
- `WsUuid`: WS-Management UUID format (`uuid:9EC885D6-F5A4-4771-9D47-4BDF7DAAEA8C`)
- `Time`: WS-Management timeout format (`PT180.000S`)
- Numeric types: `U8`, `U32`, `U64` (generated via `xml_num_value!` macro)
- `Tag<'a, V, N>`: Nested XML tags with attributes and namespace support

### XmlDeserialize Trait (`crates/xml/src/parser/mod.rs:74`)

The `XmlDeserialize` trait enables XML-to-Rust deserialization using the visitor pattern:

```rust
pub trait XmlDeserialize<'a>: Sized {
    type Visitor: XmlVisitor<'a, Value = Self>;
    fn visitor() -> Self::Visitor;
    fn from_node(node: roxmltree::Node<'a, 'a>) -> Result<Self, XmlError>;
    fn from_children(children: impl Iterator<Item = Node<'a, 'a>>) -> Result<Self, XmlError>;
}
```

**Visitor Pattern Architecture:**
- `XmlVisitor`: Traverses XML nodes and builds Rust values
- `NodeDeserializer`: Drives visitors over XML subtrees
- Each type implements both `visit_node` and `visit_children` methods
- Supports complex validation (e.g., UUID format parsing, timeout format validation)

### Bidirectional XML Processing

The architecture enables seamless round-trip XML processing:
- **Serialization**: `TagValue` converts Rust → XML
- **Deserialization**: `XmlDeserialize` converts XML → Rust
- **Type Safety**: Many types implement both traits for consistency
- **Namespace Support**: Full WS-Management, SOAP, and PowerShell namespace handling

### Supporting Infrastructure

- **TagName Trait**: Defines XML tag names and namespace URIs
- **Tag<'a, V, N>**: Generic container combining values with XML metadata (attributes, namespaces)
- **Macros**: `xml_num_value!` and `impl_xml_deserialize!` generate boilerplate implementations
- **Error Handling**: Comprehensive XML validation with descriptive error messages

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