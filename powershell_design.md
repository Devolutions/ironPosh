# PowerShell Layer Design for IronWinRM

## Design Discussion Summary

Our goal is to design the PowerShell execution layer for `ironwinrm`.

### Initial Proposal
I initially suggested a design where a `PowerShell` struct would hold a direct reference (`&'a RunspacePool`) to the `RunspacePool` it belonged to. This would have allowed for a fluent API like `PowerShell::new(&pool).add_script("...").invoke().await`.

### The Agreed-Upon Design
You correctly identified that my initial proposal was flawed. It would introduce complex lifetime management and tight coupling, which is not idiomatic or robust in Rust. You proposed a superior, decoupled design with the following key principles:

- **`PowerShell` as a Handle**: The public `PowerShell` struct should be a simple, lightweight, and copyable "handle" that contains only a `UUID` to uniquely identify a pipeline. It should not contain any complex state or references.
- **`RunspacePool` as the Central State Manager**: The `RunspacePool` must be the single source of truth. It will own and manage the complete state of all pipelines. This is achieved by maintaining an internal `HashMap`, where the key is the pipeline's `UUID` and the value is a struct (`Pipeline`) that holds all the data for that pipeline (its commands, invocation state, etc.).
- **Service-Oriented API**: All operations on a pipeline (creating it, adding commands, invoking it) will be methods on the `RunspacePool`. These methods will accept the `PowerShell` handle as an argument to identify which pipeline to operate on. This makes the `RunspacePool` a "service" that manages pipeline resources.

This refined design is more robust, avoids lifetime issues, and aligns perfectly with Rust's ownership principles.

## Detailed Implementation Skeleton

### 1. The `PowerShell` Handle
This is the simple, public-facing handle.

```rust
use uuid::Uuid;

/// A handle to a PowerShell pipeline managed by a `RunspacePool`.
///
/// This struct is a lightweight, copyable identifier for a specific pipeline.
/// All operations on the pipeline are performed via methods on the `RunspacePool`
/// that take this handle as an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PowerShell {
    pub(crate) id: Uuid,
}

impl PowerShell {
    /// Creates a new, unique PowerShell handle.
    ///
    /// This function is internal because handles should only be created by
    /// the `RunspacePool` to ensure they are tracked correctly.
    pub(crate) fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }

    /// Returns the unique identifier for this PowerShell handle.
    pub fn id(&self) -> Uuid {
        self.id
    }
}
```

### 2. The Internal `Pipeline` State
This struct holds the actual data for a pipeline and is kept private within the `RunspacePool`.

```rust
use protocol_powershell_remoting::messages::create_pipeline::PowerShellPipeline as PsPipeline;
use protocol_powershell_remoting::messages::create_pipeline::Command;
use protocol_powershell_remoting::objects::PsValue;

/// Represents the possible states of a PowerShell pipeline invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineState {
    NotStarted,
    Running,
    Stopping,
    Stopped,
    Completed,
    Failed,
}

/// Internal representation of a PowerShell pipeline's state and configuration.
/// This is owned and managed by the `RunspacePool`.
#[derive(Debug, Clone)]
pub(crate) struct Pipeline {
    pub(crate) state: PipelineState,
    pub(crate) ps_pipeline: PsPipeline,
    pub(crate) output: Vec<PsValue>,
    pub(crate) errors: Vec<PsValue>,
}

impl Pipeline {
    pub(crate) fn new() -> Self {
        Self {
            state: PipelineState::NotStarted,
            ps_pipeline: PsPipeline::default(),
            output: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub(crate) fn add_script(&mut self, script: String) {
        let command = Command::from_script(script);
        self.ps_pipeline.cmds.add_command(command);
    }

    pub(crate) fn add_command(&mut self, command: String) {
        let command = Command::from_command(command);
        self.ps_pipeline.cmds.add_command(command);
    }

    pub(crate) fn add_parameter(&mut self, name: &str, value: PsValue) {
        if let Some(last_cmd) = self.ps_pipeline.cmds.commands.last_mut() {
            last_cmd.add_parameter(name, value);
        } else {
            // In a real implementation, we might return an error here.
            // For the skeleton, we can log or panic.
            log::warn!("Attempted to add a parameter with no prior command.");
        }
    }
}
```

### 3. `RunspacePool` Modifications
The `RunspacePool` becomes the central orchestrator, managing the pipelines via its new `HashMap`.

```rust
// ...existing code...
use std::collections::HashMap;
use uuid::Uuid;
use crate::pipeline::{Pipeline, PipelineState};
use crate::powershell::PowerShell;
use crate::PwshCoreError;
use protocol_powershell_remoting::objects::PsValue;

pub struct RunspacePool {
    // ... existing fields: id, state, shell, fragmenter, etc.
    
    /// Manages the state of all pipelines associated with this pool.
    pipelines: HashMap<Uuid, Pipeline>,
}

impl RunspacePool {
    // ... existing methods: connect, close, etc. ...

    // --- PowerShell Pipeline Management API ---

    /// Creates a new PowerShell pipeline and returns a handle to it.
    ///
    /// The pipeline's state is managed internally by the `RunspacePool`.
    pub fn create_pipeline(&mut self) -> PowerShell {
        let handle = PowerShell::new();
        self.pipelines.insert(handle.id(), Pipeline::new());
        handle
    }

    /// Adds a script to the specified pipeline.
    ///
    /// # Arguments
    /// * `handle`: The handle to the pipeline to modify.
    /// * `script`: The script string to add.
    pub fn add_script(&mut self, handle: PowerShell, script: impl Into<String>) -> Result<(), PwshCoreError> {
        let pipeline = self.pipelines.get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;
        
        if pipeline.state != PipelineState::NotStarted {
            return Err(PwshCoreError::InvalidState("Cannot add to a pipeline that has already been started"));
        }

        pipeline.add_script(script.into());
        Ok(())
    }

    /// Adds a command (cmdlet) to the specified pipeline.
    pub fn add_command(&mut self, handle: PowerShell, command: impl Into<String>) -> Result<(), PwshCoreError> {
        let pipeline = self.pipelines.get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;
        
        if pipeline.state != PipelineState::NotStarted {
            return Err(PwshCoreError::InvalidState("Cannot add to a pipeline that has already been started"));
        }

        pipeline.add_command(command.into());
        Ok(())
    }

    /// Adds a parameter to the last command in the specified pipeline.
    pub fn add_parameter(&mut self, handle: PowerShell, name: &str, value: impl Into<PsValue>) -> Result<(), PwshCoreError> {
        let pipeline = self.pipelines.get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;
        
        if pipeline.state != PipelineState::NotStarted {
            return Err(PwshCoreError::InvalidState("Cannot add to a pipeline that has already been started"));
        }

        pipeline.add_parameter(name, value.into());
        Ok(())
    }

    /// Invokes the specified pipeline and waits for its completion.
    ///
    /// This method will handle the entire PSRP message exchange:
    /// 1. Send the `CreatePipeline` message.
    /// 2. Send `Command`, `Send`, and `EndOfInput` messages.
    /// 3. Enter a loop to `Receive` and process responses.
    /// 4. Defragment and deserialize messages, updating the pipeline's state, output, and error streams.
    /// 5. Return the final output upon completion.
    pub async fn invoke_pipeline(&mut self, handle: PowerShell) -> Result<&[PsValue], PwshCoreError> {
        let pipeline = self.pipelines.get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

        // --- SKELETON LOGIC ---
        // 1. Set pipeline state to Running.
        pipeline.state = PipelineState::Running;
        log::info!("Invoking pipeline {}", handle.id());

        // 2. Build the `CreatePipeline` message from `pipeline.ps_pipeline`.
        // let create_msg = ...;

        // 3. Use `self.shell` to send the command and enter the receive loop.
        // let command_id = self.shell.command(...).await?;
        // while pipeline.state == PipelineState::Running {
        //     let responses = self.shell.receive(command_id).await?;
        //     // Process responses, update pipeline.state, pipeline.output, etc.
        // }
        
        // For the skeleton, we'll just simulate a result.
        pipeline.output.push(PsValue::String("Simulated Execution Result".to_string()));
        pipeline.state = PipelineState::Completed;
        log::info!("Invocation complete for pipeline {}", handle.id());
        // --- END SKELETON LOGIC ---

        // Return a reference to the results stored within the pipeline state.
        Ok(&self.pipelines.get(&handle.id()).unwrap().output)
    }
}
```

### 4. Example Usage
This is how a consumer of your `pwsh-core` crate would use the new API.

```rust
// Example of final usage
async fn main() -> anyhow::Result<()> {
    // Assume `pool` is an initialized `RunspacePool`
    let mut pool = RunspacePool::connect(...).await?;

    // 1. Create a pipeline and get its handle.
    let ps_handle = pool.create_pipeline();

    // 2. Build the command using the handle.
    pool.add_script(ps_handle, "Get-Process")?;
    pool.add_command(ps_handle, "Sort-Object")?;
    pool.add_parameter(ps_handle, "Property", "CPU")?;

    // 3. Invoke the pipeline.
    let results = pool.invoke_pipeline(ps_handle).await?;

    println!("Pipeline finished with {} results:", results.len());
    for res in results {
        println!("> {:?}", res);
    }

    pool.close().await?;
    Ok(())
}
```

## Key Benefits of This Design

1. **Separation of Concerns**: The `PowerShell` handle is just an identifier, while the `RunspacePool` manages all state and operations.

2. **No Lifetime Issues**: Since `PowerShell` doesn't hold references, there are no complex lifetime constraints.

3. **Resource Management**: The `RunspacePool` can properly manage pipeline resources, including cleanup when pipelines complete.

4. **Thread Safety**: This design naturally supports future thread-safety enhancements, as the `RunspacePool` can control access to pipeline state.

5. **Consistency**: All pipeline operations go through the same interface, making behavior predictable and debuggable.

## Implementation Notes

- The `Pipeline` struct should be expanded to include error streams, debug streams, and other PowerShell output streams.
- The `invoke_pipeline` method needs to implement the full PSRP protocol exchange.
- Error handling should be robust, with proper cleanup of pipeline state on failure.
- Consider adding pipeline cleanup methods to remove completed pipelines from the HashMap.
