# IronPosh Client (Tokio)

An async PowerShell remoting client built with Tokio that provides both interactive and non-interactive modes for executing PowerShell commands on remote Windows machines.

## Features

- **Async/Await**: Built on Tokio for high-performance async operations
- **Interactive Mode**: REPL-style interface for interactive PowerShell sessions
- **Non-Interactive Mode**: Execute single commands and exit
- **WinRM Protocol**: Full WinRM and PowerShell Remoting protocol support
- **Structured Logging**: Comprehensive tracing with configurable verbosity levels

## Usage

### Interactive Mode

```bash
cargo run --bin ironposh-client-tokio -- -s 192.168.1.100 -u Administrator -P MyPassword
```

This starts an interactive PowerShell session where you can execute commands:

```
IronPosh Interactive PowerShell Client
Enter PowerShell commands or 'exit' to quit
PS> Get-ComputerInfo
PS> Get-Process | Select-Object -First 5
PS> exit
```

### Non-Interactive Mode

Execute a single command and exit:

```bash
cargo run --bin ironposh-client-tokio -- -s 192.168.1.100 -u Administrator -P MyPassword -c "Get-ComputerInfo"
```

### Command Line Options

- `-s, --server <IP>`: Server IP address (default: 10.10.0.3)
- `-p, --port <PORT>`: Server port (default: 5985)  
- `-u, --username <USER>`: Username for authentication (default: Administrator)
- `-P, --password <PASS>`: Password for authentication (default: DevoLabs123!)
- `--https`: Use HTTPS instead of HTTP
- `-v, --verbose`: Increase logging verbosity (can be repeated)
- `-c, --command <CMD>`: Command to execute in non-interactive mode

## Architecture

This client uses the `powershell-async` crate as its engine, which provides:

- `RemoteAsyncPowershellClient`: Main async PowerShell client
- Async HTTP client integration via `HttpClient` trait
- Connection management and message handling
- Pipeline operations for PowerShell command execution

The implementation follows the same patterns as `powershell-sync-client` but uses async/await throughout for better concurrency and performance.

## Logging

Logs are written to `ironposh_client.log` with different verbosity levels:

- `-v`: Debug level logging
- `-vv`: Trace level logging  
- `-vvv`: Full trace logging for all components

## Dependencies

- `tokio`: Async runtime
- `reqwest`: HTTP client for WinRM requests
- `powershell-async`: Async PowerShell client engine
- `pwsh-core`: Core PowerShell protocol support
- `clap`: Command line argument parsing
- `tracing`: Structured logging