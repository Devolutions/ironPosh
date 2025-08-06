# WinRM PowerShell Client Example

## Usage

The `connect.rs` example demonstrates a basic WinRM PowerShell client that connects to a remote Windows server.

### Configuration

Edit the following lines in `connect.rs` to match your test server:

```rust
let server = ServerAddress::Domain("your-server-ip".to_string()); // Change to your server
let port = 5985; // or 5986 for HTTPS
let scheme = Scheme::Http; // or Scheme::Https
let auth = Authentication::Basic {
    username: "your-username".to_string(),
    password: "your-password".to_string(),
};
```

### Running

```bash
cargo run -p pwsh-core --example connect
```

### Current Status

- ✅ Generates proper PowerShell negotiation messages (SessionCapability, InitRunspacePool)
- ✅ Fragments messages according to PowerShell remoting protocol
- ✅ Builds WS-Management SOAP envelopes
- ✅ Makes initial shell creation request
- ✅ Makes receive request
- ⚠️ ConnectReceiveCycle state hits `todo!()` (needs implementation)

### Expected Flow

1. **Idle → Connecting**: Sends shell creation request with PowerShell negotiation
2. **Connecting → ConnectReceiveCycle**: Processes shell creation response, sends receive request
3. **ConnectReceiveCycle**: Currently hits `todo!()` - needs to parse receive response and complete handshake

The client is ready for testing against real PowerShell servers and will help debug the protocol implementation step by step.