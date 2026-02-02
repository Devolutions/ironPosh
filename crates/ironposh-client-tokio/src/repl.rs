use futures::StreamExt;
use ironposh_async::RemoteAsyncPowershellClient;
use ironposh_async::SessionEvent;
use ironposh_client_core::connector::active_session::UserEvent;
use ironposh_terminal::Terminal;
use std::collections::VecDeque;
use std::fmt::Write as _;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, error, info, warn};

use crate::types::TerminalOperation;
use crate::types::{HostUiRequest, HostUiResponse, ReplControl};

fn clamp_console_color(color: i32) -> i32 {
    color.clamp(0, 15)
}

fn sgr_for_foreground(color: i32) -> i32 {
    match clamp_console_color(color) {
        0 => 30,  // Black
        1 => 34,  // DarkBlue
        2 => 32,  // DarkGreen
        3 => 36,  // DarkCyan
        4 => 31,  // DarkRed
        5 => 35,  // DarkMagenta
        6 => 33,  // DarkYellow
        7 => 37,  // Gray
        8 => 90,  // DarkGray
        9 => 94,  // Blue
        10 => 92, // Green
        11 => 96, // Cyan
        12 => 91, // Red
        13 => 95, // Magenta
        14 => 93, // Yellow
        15 => 97, // White
        _ => unreachable!("color is clamped to 0..15"),
    }
}

fn sgr_for_background(color: i32) -> i32 {
    match clamp_console_color(color) {
        0 => 40,   // Black
        1 => 44,   // DarkBlue
        2 => 42,   // DarkGreen
        3 => 46,   // DarkCyan
        4 => 41,   // DarkRed
        5 => 45,   // DarkMagenta
        6 => 43,   // DarkYellow
        7 => 47,   // Gray
        8 => 100,  // DarkGray
        9 => 104,  // Blue
        10 => 102, // Green
        11 => 106, // Cyan
        12 => 101, // Red
        13 => 105, // Magenta
        14 => 103, // Yellow
        15 => 107, // White
        _ => unreachable!("color is clamped to 0..15"),
    }
}

fn format_host_information_message(msg: &ironposh_psrp::HostInformationMessage) -> (String, bool) {
    let mut out = String::new();
    if let Some(fg) = msg.foreground_color {
        let _ = write!(&mut out, "\x1b[{}m", sgr_for_foreground(fg));
    }
    if let Some(bg) = msg.background_color {
        let _ = write!(&mut out, "\x1b[{}m", sgr_for_background(bg));
    }
    out.push_str(&msg.message);
    if msg.foreground_color.is_some() || msg.background_color.is_some() {
        out.push_str("\x1b[0m");
    }
    (out, !msg.no_new_line)
}

#[derive(Debug)]
enum UserInput {
    Cmd(String),
    Interrupt,
    Eof,
}

async fn run_script_and_forward_nested(
    client: &mut RemoteAsyncPowershellClient,
    terminal_op_tx: &Sender<TerminalOperation>,
    cmd: String,
    user_input_rx: &mut Receiver<UserInput>,
    repl_control_rx: &mut Receiver<ReplControl>,
) -> anyhow::Result<()> {
    info!(command = %cmd, "Sending command to PowerShell (nested)");
    let stream = client.send_script(cmd).await?;
    let mut stream = stream.boxed();
    let mut pipeline: Option<ironposh_client_core::powershell::PipelineHandle> = None;

    loop {
        tokio::select! {
            Some(ctrl) = repl_control_rx.recv() => {
                match ctrl {
                    ReplControl::ExitNestedPrompt => {
                        info!("ExitNestedPrompt received while pipeline running");
                        if let Some(h) = pipeline.take() {
                            client.kill_pipeline(h).await?;
                        }
                        break;
                    }
                    ReplControl::ShouldExit(code) => {
                        warn!(code, "ShouldExit received while pipeline running");
                        if let Some(h) = pipeline.take() {
                            client.kill_pipeline(h).await?;
                        }
                        return Err(anyhow::anyhow!("remote requested exit (code {code})"));
                    }
                    ReplControl::EnterNestedPrompt => {
                        debug!("EnterNestedPrompt received while already nested (ignored)");
                    }
                }
            }
            Some(ui_msg) = user_input_rx.recv() => {
                if matches!(ui_msg, UserInput::Interrupt) {
                    if let Some(h) = pipeline.take() {
                        info!("Interrupt received; killing pipeline");
                        client.kill_pipeline(h).await?;
                    }
                    break;
                }
            }
            Some(ev) = stream.next() => {
                let _ = terminal_op_tx.send(TerminalOperation::CheckInterrupt).await;
                match ev {
                    UserEvent::PipelineCreated { pipeline: p } => {
                        pipeline = Some(p);
                    }
                    UserEvent::PipelineFinished { .. } => {
                        break;
                    }
                    UserEvent::PipelineOutput { output, .. } => {
                        let text = output
                            .format_as_displyable_string()
                            .unwrap_or_else(|e| format!("Error formatting output: {e}"));
                        let _ = terminal_op_tx.send(TerminalOperation::Print(text)).await;
                    }
                    UserEvent::ErrorRecord { error_record, .. } => {
                        let _ = terminal_op_tx
                            .send(TerminalOperation::Print(format!(
                                "Error: {}",
                                error_record.render_concise()
                            )))
                            .await;
                    }
                    UserEvent::PipelineRecord { record, .. } => {
                        use ironposh_client_core::psrp_record::PsrpRecord;
                        match record {
                            PsrpRecord::Debug { message, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!("Debug: {message}")))
                                    .await;
                            }
                            PsrpRecord::Verbose { message, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!("Verbose: {message}")))
                                    .await;
                            }
                            PsrpRecord::Warning { message, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!("Warning: {message}")))
                                    .await;
                            }
                            PsrpRecord::Information { record, .. } => {
                                match &record.message_data {
                                    ironposh_psrp::InformationMessageData::HostInformationMessage(m) => {
                                        let (text, newline) = format_host_information_message(m);
                                        let _ = terminal_op_tx
                                            .send(TerminalOperation::Write { text, newline })
                                            .await;
                                    }
                                    ironposh_psrp::InformationMessageData::String(s) => {
                                        let _ = terminal_op_tx
                                            .send(TerminalOperation::Print(format!(
                                                "[information] {s}"
                                            )))
                                            .await;
                                    }
                                    ironposh_psrp::InformationMessageData::Object(v) => {
                                        let _ = terminal_op_tx
                                            .send(TerminalOperation::Print(format!(
                                                "[information] {v}"
                                            )))
                                            .await;
                                    }
                                }
                            }
                            PsrpRecord::Progress { record, .. } => {
                                let status = record.status_description.clone().unwrap_or_default();
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!(
                                        "[progress] {}: {} ({}%)",
                                        record.activity, status, record.percent_complete
                                    )))
                                    .await;
                            }
                            PsrpRecord::Unsupported { data_preview, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!(
                                        "[unsupported] {data_preview}"
                                    )))
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_nested_prompt_loop(
    client: &mut RemoteAsyncPowershellClient,
    terminal_op_tx: &Sender<TerminalOperation>,
    user_input_rx: &mut Receiver<UserInput>,
    repl_control_rx: &mut Receiver<ReplControl>,
) -> anyhow::Result<()> {
    info!("Entering nested prompt mode");
    let _ = terminal_op_tx
        .send(TerminalOperation::Print(
            "Entering nested prompt (type 'exit' to leave)".to_string(),
        ))
        .await;

    loop {
        let _ = terminal_op_tx
            .send(TerminalOperation::RequestInput {
                prompt: "NESTED> ".to_string(),
            })
            .await;

        tokio::select! {
            Some(ctrl) = repl_control_rx.recv() => {
                match ctrl {
                    ReplControl::ExitNestedPrompt => {
                        info!("ExitNestedPrompt received; leaving nested prompt mode");
                        break;
                    }
                    ReplControl::ShouldExit(code) => {
                        warn!(code, "ShouldExit received; leaving nested prompt mode");
                        return Err(anyhow::anyhow!("remote requested exit (code {code})"));
                    }
                    ReplControl::EnterNestedPrompt => {
                        debug!("EnterNestedPrompt received while already nested (ignored)");
                    }
                }
            }
            Some(msg) = user_input_rx.recv() => {
                match msg {
                    UserInput::Cmd(cmd) => {
                        let cmd = cmd.trim().to_string();
                        if cmd.eq_ignore_ascii_case("exit") {
                            break;
                        }
                        if cmd.is_empty() {
                            continue;
                        }
                        run_script_and_forward_nested(client, terminal_op_tx, cmd, user_input_rx, repl_control_rx).await?;
                    }
                    UserInput::Interrupt => {
                        // just reprompt
                    }
                    UserInput::Eof => break,
                }
            }
        }
    }

    let _ = terminal_op_tx
        .send(TerminalOperation::Print(
            "Leaving nested prompt.".to_string(),
        ))
        .await;

    Ok(())
}

fn sanitize_prompt(mut prompt: String) -> String {
    // Some prompts may contain newlines; keep only the last line for a single-line UI prompt.
    if prompt.contains('\n') || prompt.contains('\r') {
        prompt = prompt.lines().last().unwrap_or("").to_string();
    }

    prompt
}

async fn fetch_remote_prompt(client: &mut RemoteAsyncPowershellClient) -> Option<String> {
    // Use PowerShell's `prompt` function so user customizations ($PROFILE, etc.) are reflected.
    let mut stream = match client.send_script("prompt".to_string()).await {
        Ok(stream) => stream.boxed(),
        Err(e) => {
            warn!(error = %e, "failed to request remote prompt; falling back");
            return None;
        }
    };

    let mut last_prompt: Option<String> = None;

    while let Some(ev) = stream.next().await {
        match ev {
            UserEvent::PipelineOutput { output, .. } => {
                match output.format_as_displyable_string() {
                    Ok(text) => {
                        if !text.trim().is_empty() {
                            last_prompt = Some(text);
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to parse remote prompt output");
                    }
                }
            }
            UserEvent::ErrorRecord { error_record, .. } => {
                warn!(error = %error_record.render_concise(), "remote prompt command returned an error");
            }
            UserEvent::PipelineFinished { .. } => break,
            UserEvent::PipelineCreated { .. } | UserEvent::PipelineRecord { .. } => {}
        }
    }

    let prompt = last_prompt
        .map(sanitize_prompt)
        .filter(|s| !s.trim().is_empty());
    debug!(prompt = ?prompt, "remote prompt fetched");
    prompt
}

async fn request_prompt(
    client: &mut RemoteAsyncPowershellClient,
    terminal_op_tx: &Sender<TerminalOperation>,
) {
    // Important: many customized prompts use `Write-Host` and return an empty
    // string. In that case, the prompt has already been rendered via HostCalls,
    // and we should not print any extra local prompt.
    let prompt = fetch_remote_prompt(client).await.unwrap_or_default();
    let _ = terminal_op_tx
        .send(TerminalOperation::RequestInput { prompt })
        .await;
}

async fn wait_for_active_session(
    session_event_rx: &mut futures::channel::mpsc::UnboundedReceiver<SessionEvent>,
) -> anyhow::Result<()> {
    while let Some(ev) = session_event_rx.next().await {
        match ev {
            SessionEvent::ActiveSessionStarted => {
                info!("Active session started");
                return Ok(());
            }
            SessionEvent::Error(e) => {
                return Err(anyhow::anyhow!("Session error: {e}"));
            }
            other => {
                debug!(event = ?other, "session event");
            }
        }
    }

    Err(anyhow::anyhow!(
        "Session event channel closed before ActiveSessionStarted"
    ))
}

/// Run the UI thread that owns the terminal and processes UI operations
fn run_ui_thread(
    mut terminal: Terminal,
    mut terminal_op_rx: Receiver<TerminalOperation>,
    user_input_tx: Sender<UserInput>,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use ironposh_terminal::ReadOutcome;
        use ironposh_terminal::TerminalOp;
        use std::io::Write;

        fn vt_color_to_console(color: vt100::Color, default: i32) -> i32 {
            match color {
                vt100::Color::Default => default,
                vt100::Color::Idx(i) => i32::from(i).clamp(0, 15),
                vt100::Color::Rgb(r, g, b) => {
                    const PALETTE: [(i32, (u8, u8, u8)); 16] = [
                        (0, (0, 0, 0)),
                        (1, (0, 0, 128)),
                        (2, (0, 128, 0)),
                        (3, (0, 128, 128)),
                        (4, (128, 0, 0)),
                        (5, (128, 0, 128)),
                        (6, (128, 128, 0)),
                        (7, (192, 192, 192)),
                        (8, (128, 128, 128)),
                        (9, (0, 0, 255)),
                        (10, (0, 255, 0)),
                        (11, (0, 255, 255)),
                        (12, (255, 0, 0)),
                        (13, (255, 0, 255)),
                        (14, (255, 255, 0)),
                        (15, (255, 255, 255)),
                    ];

                    let mut best = default.clamp(0, 15);
                    let mut best_dist = u32::MAX;
                    for (idx, (pr, pg, pb)) in PALETTE {
                        let dr = i32::from(r).abs_diff(i32::from(pr));
                        let dg = i32::from(g).abs_diff(i32::from(pg));
                        let db = i32::from(b).abs_diff(i32::from(pb));
                        let dist = dr * dr + dg * dg + db * db;
                        if dist < best_dist {
                            best_dist = dist;
                            best = idx;
                        }
                    }
                    best
                }
            }
        }

        fn secure_string_bytes(s: &str) -> Vec<u8> {
            s.encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<u8>>()
        }

        fn read_secure_line(
            io: &mut ironposh_terminal::StdTerm<'_>,
            prompt: &str,
            event_queue: &mut VecDeque<crossterm::event::Event>,
        ) -> std::io::Result<String> {
            if !prompt.is_empty() {
                io.write_all(b"\r")?;
                io.write_all(prompt.as_bytes())?;
                io.flush()?;
            }

            let mut line = String::new();

            loop {
                let evt = if let Some(evt) = event_queue.pop_front() {
                    Some(evt)
                } else if crossterm::event::poll(std::time::Duration::from_millis(50))? {
                    Some(crossterm::event::read()?)
                } else {
                    None
                };

                if let Some(evt) = evt {
                    match evt {
                        crossterm::event::Event::Resize(cols, rows) => {
                            io.apply_op(TerminalOp::Resize { rows, cols });
                            io.render().map_err(std::io::Error::other)?;
                        }
                        crossterm::event::Event::Key(crossterm::event::KeyEvent {
                            kind: crossterm::event::KeyEventKind::Press,
                            code: crossterm::event::KeyCode::Enter,
                            ..
                        }) => {
                            io.write_all(b"\r\n")?;
                            io.flush()?;
                            return Ok(line);
                        }
                        crossterm::event::Event::Key(crossterm::event::KeyEvent {
                            kind: crossterm::event::KeyEventKind::Press,
                            code: crossterm::event::KeyCode::Backspace,
                            ..
                        }) => {
                            if !line.is_empty() {
                                line.pop();
                                io.write_all(b"\x08 \x08")?;
                                io.flush()?;
                            }
                        }
                        crossterm::event::Event::Key(crossterm::event::KeyEvent {
                            kind: crossterm::event::KeyEventKind::Press,
                            code: crossterm::event::KeyCode::Char('c'),
                            modifiers,
                            ..
                        }) if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            io.write_all(b"^C\r\n")?;
                            io.flush()?;
                            return Ok(String::new());
                        }
                        crossterm::event::Event::Key(crossterm::event::KeyEvent {
                            kind: crossterm::event::KeyEventKind::Press,
                            code: crossterm::event::KeyCode::Char(c),
                            modifiers,
                            ..
                        }) if !modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            line.push(c);
                            io.write_all(b"*")?;
                            io.flush()?;
                        }
                        crossterm::event::Event::Paste(s) => {
                            line.push_str(&s);
                            for _ in s.chars() {
                                io.write_all(b"*")?;
                            }
                            io.flush()?;
                        }
                        _ => {}
                    }
                }

                if io.render().is_err() {
                    // best-effort render; ignore in input loop
                }
            }
        }

        info!("UI thread starting with unified queue");
        let mut io = terminal.stdio();
        let mut event_queue: VecDeque<crossterm::event::Event> = VecDeque::new();

        let _ui = tracing::span!(tracing::Level::INFO, "UI Thread").entered();
        // Drain all pending UI ops
        while let Some(op) = terminal_op_rx.blocking_recv() {
            info!(op = ?op, "Processing terminal operation");
            match op {
                TerminalOperation::Apply(ops) => {
                    debug!(count = ops.len(), "applying terminal operations");
                    for o in ops {
                        io.apply_op(o);
                    }
                    if let Err(e) = io.render() {
                        error!(error = %e, "failed to render terminal");
                        return Err(e);
                    }
                }
                TerminalOperation::Print(s) => {
                    debug!(chars = s.len(), "printing output");
                    if let Err(e) = writeln!(io, "{s}") {
                        error!(error = %e, "failed to write to terminal");
                        return Err(e.into());
                    }
                    if let Err(e) = io.render() {
                        error!(error = %e, "failed to render terminal after print");
                        return Err(e);
                    }
                }
                TerminalOperation::Write { text, newline } => {
                    debug!(chars = text.len(), newline, "writing output");
                    if newline {
                        if let Err(e) = writeln!(io, "{text}") {
                            error!(error = %e, "failed to write line to terminal");
                            return Err(e.into());
                        }
                    } else if let Err(e) = write!(io, "{text}") {
                        error!(error = %e, "failed to write to terminal");
                        return Err(e.into());
                    }

                    if let Err(e) = io.render() {
                        error!(error = %e, "failed to render terminal after write");
                        return Err(e);
                    }
                }
                TerminalOperation::SetWindowTitle { title } => {
                    debug!(title = %title, "setting host window title");
                    // Best-effort: write directly to the host terminal using Crossterm.
                    // Do not route through the guest terminal emulator.
                    if let Err(e) =
                        crossterm::execute!(std::io::stdout(), crossterm::terminal::SetTitle(title))
                    {
                        warn!(error = %e, "failed to set window title");
                    }
                }
                TerminalOperation::RequestInput { prompt } => {
                    debug!(prompt = %prompt, "reading user input");
                    match io.read_line_queued(&prompt, &mut event_queue) {
                        Ok(ReadOutcome::Line(s)) => {
                            info!(command = %s.trim(), "user entered command");
                            if user_input_tx.blocking_send(UserInput::Cmd(s)).is_err() {
                                warn!("failed to send command to REPL - channel closed");
                                return Ok(());
                            }
                        }
                        Ok(ReadOutcome::Interrupt) => {
                            info!("user pressed Ctrl+C");
                            if user_input_tx.blocking_send(UserInput::Interrupt).is_err() {
                                warn!("failed to send interrupt to REPL - channel closed");
                                return Ok(());
                            }
                        }
                        Ok(ReadOutcome::Eof) => {
                            info!("received EOF from user input");
                            let _ = user_input_tx.blocking_send(UserInput::Eof);
                            return Ok(());
                        }
                        Err(e) => {
                            error!(error = %e, "error reading user input");
                            return Err(e.into());
                        }
                    }
                }
                TerminalOperation::CheckInterrupt => {
                    if let Some(read_line) = io.try_read_line_queued(&mut event_queue)? {
                        match read_line {
                            ReadOutcome::Line(s) => {
                                info!(command = %s.trim(), "user entered command");
                                if user_input_tx.blocking_send(UserInput::Cmd(s)).is_err() {
                                    warn!("failed to send command to REPL - channel closed");
                                    return Ok(());
                                }
                            }
                            ReadOutcome::Interrupt => {
                                info!("user pressed Ctrl+C");
                                if user_input_tx.blocking_send(UserInput::Interrupt).is_err() {
                                    warn!("failed to send interrupt to REPL - channel closed");
                                    return Ok(());
                                }
                            }
                            ReadOutcome::Eof => {
                                info!("received EOF from user input");
                                let _ = user_input_tx.blocking_send(UserInput::Eof);
                                return Ok(());
                            }
                        }
                    }
                }
                TerminalOperation::HostRequest {
                    request,
                    respond_to,
                } => {
                    let response = match request {
                        HostUiRequest::ReadLine => {
                            match io.read_line_queued("", &mut event_queue)? {
                                ReadOutcome::Line(s) => HostUiResponse::Line(s),
                                ReadOutcome::Interrupt | ReadOutcome::Eof => {
                                    HostUiResponse::Line(String::new())
                                }
                            }
                        }
                        HostUiRequest::ReadLineAsSecureString => {
                            let s = read_secure_line(&mut io, "", &mut event_queue)?;
                            HostUiResponse::SecureBytes(secure_string_bytes(&s))
                        }
                        HostUiRequest::Prompt {
                            caption,
                            message,
                            fields,
                        } => {
                            if !caption.trim().is_empty() {
                                let _ = writeln!(io, "{caption}");
                            }
                            if !message.trim().is_empty() {
                                let _ = writeln!(io, "{message}");
                            }

                            let mut out = std::collections::HashMap::new();
                            for field in fields {
                                loop {
                                    let label = if field.label.trim().is_empty() {
                                        field.name.clone()
                                    } else {
                                        field.label.clone()
                                    };
                                    let prompt = format!("{label}: ");

                                    let is_secure = field.parameter_type.contains("SecureString")
                                        || field
                                            .parameter_type
                                            .contains("System.Security.SecureString");

                                    let input = if is_secure {
                                        read_secure_line(&mut io, &prompt, &mut event_queue)?
                                    } else {
                                        let ReadOutcome::Line(input) =
                                            io.read_line_queued(&prompt, &mut event_queue)?
                                        else {
                                            continue;
                                        };
                                        input
                                    };
                                    let input = input.trim().to_string();

                                    if input.is_empty() {
                                        if let Some(default_value) = field.default_value.clone() {
                                            out.insert(field.name.clone(), default_value);
                                            break;
                                        }
                                        if field.is_mandatory {
                                            let _ = writeln!(io, "Value is required.");
                                            continue;
                                        }
                                        out.insert(
                                            field.name.clone(),
                                            if is_secure {
                                                ironposh_psrp::PsValue::Primitive(
                                                    ironposh_psrp::PsPrimitiveValue::SecureString(
                                                        Vec::new(),
                                                    ),
                                                )
                                            } else {
                                                ironposh_psrp::PsValue::from(String::new())
                                            },
                                        );
                                        break;
                                    }

                                    let ps_val = if is_secure {
                                        ironposh_psrp::PsValue::Primitive(
                                            ironposh_psrp::PsPrimitiveValue::SecureString(
                                                secure_string_bytes(&input),
                                            ),
                                        )
                                    } else if field.parameter_type.contains("Int") {
                                        input.parse::<i32>().map_or_else(
                                            |_| ironposh_psrp::PsValue::from(input),
                                            ironposh_psrp::PsValue::from,
                                        )
                                    } else if field.parameter_type.contains("Bool") {
                                        let v = matches!(
                                            input.to_ascii_lowercase().as_str(),
                                            "1" | "true" | "t" | "yes" | "y"
                                        );
                                        ironposh_psrp::PsValue::from(v)
                                    } else {
                                        ironposh_psrp::PsValue::from(input)
                                    };
                                    out.insert(field.name.clone(), ps_val);
                                    break;
                                }
                            }

                            HostUiResponse::PromptResult(out)
                        }
                        HostUiRequest::PromptForChoice {
                            caption,
                            message,
                            choices,
                            default_choice,
                        } => {
                            if !caption.trim().is_empty() {
                                let _ = writeln!(io, "{caption}");
                            }
                            if !message.trim().is_empty() {
                                let _ = writeln!(io, "{message}");
                            }

                            for (idx, choice) in choices.iter().enumerate() {
                                let label = choice.label.replace('&', "");
                                let _ = writeln!(io, "[{idx}] {label}");
                            }

                            let prompt = format!("Choice (default {default_choice}): ");
                            loop {
                                let ReadOutcome::Line(input) =
                                    io.read_line_queued(&prompt, &mut event_queue)?
                                else {
                                    continue;
                                };
                                let input = input.trim();
                                if input.is_empty() {
                                    break HostUiResponse::Choice(default_choice);
                                }
                                if let Ok(idx) = input.parse::<i32>() {
                                    if idx >= 0 && (idx as usize) < choices.len() {
                                        break HostUiResponse::Choice(idx);
                                    }
                                }
                                let ch = input.chars().next().unwrap_or_default();
                                let mut matched = None;
                                for (idx, choice) in choices.iter().enumerate() {
                                    let label = &choice.label;
                                    let hot = label
                                        .chars()
                                        .collect::<Vec<_>>()
                                        .windows(2)
                                        .find_map(|w| (w[0] == '&').then_some(w[1]))
                                        .unwrap_or_else(|| {
                                            label.chars().next().unwrap_or_default()
                                        });
                                    if hot.eq_ignore_ascii_case(&ch) {
                                        matched = Some(idx as i32);
                                        break;
                                    }
                                }
                                if let Some(idx) = matched {
                                    break HostUiResponse::Choice(idx);
                                }
                                let _ = writeln!(io, "Invalid selection.");
                            }
                        }
                        HostUiRequest::PromptForChoiceMultipleSelection {
                            caption,
                            message,
                            choices,
                            default_choices,
                        } => {
                            if !caption.trim().is_empty() {
                                let _ = writeln!(io, "{caption}");
                            }
                            if !message.trim().is_empty() {
                                let _ = writeln!(io, "{message}");
                            }

                            for (idx, choice) in choices.iter().enumerate() {
                                let label = choice.label.replace('&', "");
                                let _ = writeln!(io, "[{idx}] {label}");
                            }

                            let prompt = format!(
                                "Choices (comma-separated, default {}): ",
                                default_choices
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(",")
                            );
                            loop {
                                let ReadOutcome::Line(input) =
                                    io.read_line_queued(&prompt, &mut event_queue)?
                                else {
                                    continue;
                                };
                                let input = input.trim();
                                if input.is_empty() {
                                    break HostUiResponse::ChoiceMulti(default_choices);
                                }

                                let mut selections = Vec::new();
                                let mut ok = true;
                                for part in input.split([',', ' ']) {
                                    let part = part.trim();
                                    if part.is_empty() {
                                        continue;
                                    }
                                    match part.parse::<i32>() {
                                        Ok(idx) if idx >= 0 && (idx as usize) < choices.len() => {
                                            if !selections.contains(&idx) {
                                                selections.push(idx);
                                            }
                                        }
                                        _ => {
                                            ok = false;
                                            break;
                                        }
                                    }
                                }

                                if ok {
                                    break HostUiResponse::ChoiceMulti(selections);
                                }
                                let _ = writeln!(io, "Invalid selection(s).");
                            }
                        }
                        HostUiRequest::PromptForCredential1 {
                            caption,
                            message,
                            user_name,
                            target_name,
                        } => {
                            if !caption.trim().is_empty() {
                                let _ = writeln!(io, "{caption}");
                            }
                            if !message.trim().is_empty() {
                                let _ = writeln!(io, "{message}");
                            }
                            if !target_name.trim().is_empty() {
                                let _ = writeln!(io, "Target: {target_name}");
                            }

                            let user_prompt = if user_name.trim().is_empty() {
                                "User: ".to_string()
                            } else {
                                format!("User [{user_name}]: ")
                            };
                            let user = match io.read_line_queued(&user_prompt, &mut event_queue)? {
                                ReadOutcome::Line(u) => {
                                    let u = u.trim().to_string();
                                    if u.is_empty() {
                                        user_name
                                    } else {
                                        u
                                    }
                                }
                                ReadOutcome::Interrupt | ReadOutcome::Eof => user_name,
                            };

                            let pw = read_secure_line(&mut io, "Password: ", &mut event_queue)?;
                            HostUiResponse::Credential(ironposh_client_core::host::PSCredential {
                                user_name: user,
                                password: secure_string_bytes(&pw),
                            })
                        }
                        HostUiRequest::PromptForCredential2 {
                            caption,
                            message,
                            user_name,
                            target_name,
                            allowed_credential_types,
                            options,
                        } => {
                            if !caption.trim().is_empty() {
                                let _ = writeln!(io, "{caption}");
                            }
                            if !message.trim().is_empty() {
                                let _ = writeln!(io, "{message}");
                            }
                            if !target_name.trim().is_empty() {
                                let _ = writeln!(io, "Target: {target_name}");
                            }
                            let _ = writeln!(
                                io,
                                "AllowedCredentialTypes: {allowed_credential_types}, Options: {options}"
                            );

                            let user_prompt = if user_name.trim().is_empty() {
                                "User: ".to_string()
                            } else {
                                format!("User [{user_name}]: ")
                            };
                            let user = match io.read_line_queued(&user_prompt, &mut event_queue)? {
                                ReadOutcome::Line(u) => {
                                    let u = u.trim().to_string();
                                    if u.is_empty() {
                                        user_name
                                    } else {
                                        u
                                    }
                                }
                                ReadOutcome::Interrupt | ReadOutcome::Eof => user_name,
                            };

                            let pw = read_secure_line(&mut io, "Password: ", &mut event_queue)?;
                            HostUiResponse::Credential(ironposh_client_core::host::PSCredential {
                                user_name: user,
                                password: secure_string_bytes(&pw),
                            })
                        }
                        HostUiRequest::ReadKey { options } => {
                            let no_echo = (options & 0b0100) != 0;
                            let key = loop {
                                if let Some(evt) = event_queue.pop_front() {
                                    if let crossterm::event::Event::Key(k) = evt {
                                        break k;
                                    }
                                    continue;
                                }
                                let evt = crossterm::event::read()?;
                                if let crossterm::event::Event::Key(k) = evt {
                                    break k;
                                }
                            };

                            let (vk, ch) = match key.code {
                                crossterm::event::KeyCode::Char(c) => {
                                    (i32::from(c.to_ascii_uppercase() as u16), c)
                                }
                                crossterm::event::KeyCode::Enter => (13, '\r'),
                                crossterm::event::KeyCode::Backspace => (8, '\u{8}'),
                                crossterm::event::KeyCode::Tab => (9, '\t'),
                                crossterm::event::KeyCode::Esc => (27, '\u{1b}'),
                                crossterm::event::KeyCode::Left => (37, '\0'),
                                crossterm::event::KeyCode::Up => (38, '\0'),
                                crossterm::event::KeyCode::Right => (39, '\0'),
                                crossterm::event::KeyCode::Down => (40, '\0'),
                                _ => (0, '\0'),
                            };

                            let mut control_key_state = 0;
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::SHIFT)
                            {
                                control_key_state |= 0x0010;
                            }
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                control_key_state |= 0x0008;
                            }
                            if key.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
                                control_key_state |= 0x0002;
                            }

                            if !no_echo && ch != '\0' {
                                let _ = write!(io, "{ch}");
                                let _ = io.flush();
                            }

                            HostUiResponse::KeyInfo(ironposh_client_core::host::KeyInfo {
                                virtual_key_code: vk,
                                character: ch,
                                control_key_state,
                                key_down: true,
                            })
                        }
                        HostUiRequest::GetKeyAvailable => {
                            // Try to read one pending event into the queue so we can "peek".
                            if crossterm::event::poll(std::time::Duration::from_millis(0))? {
                                let evt = crossterm::event::read()?;
                                match evt {
                                    crossterm::event::Event::Resize(cols, rows) => {
                                        io.apply_op(TerminalOp::Resize { rows, cols });
                                        let _ = io.render();
                                    }
                                    other => event_queue.push_back(other),
                                }
                            }

                            let has_key = event_queue
                                .iter()
                                .any(|e| matches!(e, crossterm::event::Event::Key(_)));
                            HostUiResponse::Bool(has_key)
                        }
                        HostUiRequest::FlushInputBuffer => {
                            event_queue.clear();
                            while crossterm::event::poll(std::time::Duration::from_millis(0))? {
                                let _ = crossterm::event::read()?;
                            }
                            HostUiResponse::Unit
                        }
                        HostUiRequest::GetCursorPosition => {
                            let (row, col) = io.guest_cursor_position();
                            HostUiResponse::CursorPosition(
                                ironposh_client_core::host::Coordinates {
                                    x: col as i32,
                                    y: row as i32,
                                },
                            )
                        }
                        HostUiRequest::GetBufferContents { rect } => {
                            let (rows, cols) = io.guest_screen_size();
                            let left = rect.left.max(0) as u16;
                            let top = rect.top.max(0) as u16;
                            let right = rect.right.max(0) as u16;
                            let bottom = rect.bottom.max(0) as u16;

                            let mut out = Vec::new();
                            if !(left >= cols || top >= rows || right < left || bottom < top) {
                                let right = right.min(cols.saturating_sub(1));
                                let bottom = bottom.min(rows.saturating_sub(1));

                                for r in top..=bottom {
                                    let mut row_vec = Vec::new();
                                    for c in left..=right {
                                        let cell = io.guest_cell(r, c);
                                        let (ch, fg, bg) = cell.map_or((' ', 7, 0), |cell| {
                                            let ch = cell.contents().chars().next().unwrap_or(' ');
                                            let fg = vt_color_to_console(cell.fgcolor(), 7);
                                            let bg = vt_color_to_console(cell.bgcolor(), 0);
                                            (ch, fg, bg)
                                        });

                                        row_vec.push(ironposh_client_core::host::BufferCell {
                                            character: ch,
                                            foreground: fg,
                                            background: bg,
                                            flags: 0,
                                        });
                                    }
                                    out.push(row_vec);
                                }
                            }

                            HostUiResponse::BufferContents(out)
                        }
                    };

                    // Best-effort: if the receiver is gone, ignore.
                    let _ = respond_to.send(response);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        Ok(())
    })
}

/// Run the main REPL event loop
async fn run_repl_loop(
    client: &mut RemoteAsyncPowershellClient,
    terminal_op_tx: Sender<TerminalOperation>,
    mut user_input_rx: Receiver<UserInput>,
    mut repl_control_rx: Receiver<ReplControl>,
) -> anyhow::Result<()> {
    info!("Starting unified REPL loop");

    // Ask for the first prompt
    request_prompt(client, &terminal_op_tx).await;

    // Async REPL loop
    let mut current_pipeline = None;
    let mut current_stream = None::<futures::stream::BoxStream<'_, UserEvent>>;

    loop {
        tokio::select! {
            Some(ctrl) = repl_control_rx.recv() => {
                match ctrl {
                    ReplControl::EnterNestedPrompt => {
                        current_pipeline = None;
                        current_stream = None;
                        run_nested_prompt_loop(
                            client,
                            &terminal_op_tx,
                            &mut user_input_rx,
                            &mut repl_control_rx,
                        )
                        .await?;
                        request_prompt(client, &terminal_op_tx).await;
                    }
                    ReplControl::ExitNestedPrompt => {
                        debug!("ExitNestedPrompt received while not nested (ignored)");
                    }
                    ReplControl::ShouldExit(code) => {
                        warn!(code, "ShouldExit received; terminating REPL");
                        break;
                    }
                }
            }
            // User input from UI thread
            Some(msg) = user_input_rx.recv() => {
                debug!(?msg, "Received message from UI thread");
                match msg {
                    UserInput::Eof => {
                        info!("Received EOF, exiting REPL");
                        break;
                    }
                    UserInput::Interrupt => {
                        if let Some(h) = current_pipeline.take() {
                            info!(pipeline = ?h, "Killing active pipeline due to interrupt");
                            client.kill_pipeline(h).await?;
                            current_stream = None;
                        }
                        request_prompt(client, &terminal_op_tx).await;
                    }
                    UserInput::Cmd(cmd) => {
                        let cmd = cmd.trim().to_string();
                        info!(command = %cmd, "processing command");

                        if cmd.eq_ignore_ascii_case("exit") {
                            info!("Exit command received, terminating REPL");
                            break;
                        }

                        if cmd.is_empty() {
                            debug!("Empty command, requesting new prompt");
                            request_prompt(client, &terminal_op_tx).await;
                            continue;
                        }

                        // Start a pipeline
                        info!(command = %cmd, "Sending command to PowerShell");
                        match client.send_script(cmd).await {
                            Ok(stream) => {
                                info!("Command sent successfully, waiting for events");
                                current_stream = Some(stream.boxed());
                                current_pipeline = None; // will be set on PipelineCreated
                            }
                            Err(e) => {
                                error!("Failed to send command: {}", e);
                                let _ = terminal_op_tx.send(TerminalOperation::Print(format!("Error sending command: {e}"))).await;
                                request_prompt(client, &terminal_op_tx).await;
                            }
                        }
                    }
                }
            }

            // Pipeline events
            Some(ev) = async {
                match &mut current_stream {
                    Some(s) => s.next().await,
                    None => futures::future::pending().await,
                }
            } => {
                debug!(?ev,"Received pipeline event");
                let _ = terminal_op_tx.send(TerminalOperation::CheckInterrupt).await;
                match ev {
                    UserEvent::PipelineCreated { pipeline } => {
                        info!(pipeline = ?pipeline, "Pipeline created");
                        current_pipeline = Some(pipeline);
                    }
                    UserEvent::PipelineFinished { .. } => {
                        info!("Pipeline finished");
                        current_pipeline = None;
                        current_stream = None;
                        // Request new prompt after pipeline finishes
                        request_prompt(client, &terminal_op_tx).await;
                    }
                    UserEvent::PipelineOutput { output, .. } => {
                        debug!("Received pipeline output");
                        let text = match output.format_as_displyable_string() {
                            Ok(s) => {
                                debug!("Formatted output: {} chars", s.len());
                                s
                            }
                            Err(e) => {
                                error!("Error formatting output: {}", e);
                                format!("Error formatting output: {e}")
                            }
                        };
                        let _ = terminal_op_tx.send(TerminalOperation::Print(text)).await;
                    }
                    UserEvent::ErrorRecord { error_record, .. } => {
                        debug!("Received error record");
                        let error_text = error_record.render_concise();
                        let _ = terminal_op_tx.send(TerminalOperation::Print(format!("Error: {error_text}"))).await;
                    }
                    UserEvent::PipelineRecord { record, .. } => {
                        use ironposh_client_core::psrp_record::PsrpRecord;
                        match record {
                            PsrpRecord::Debug { message, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!("Debug: {message}")))
                                    .await;
                            }
                            PsrpRecord::Verbose { message, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!("Verbose: {message}")))
                                    .await;
                            }
                            PsrpRecord::Warning { message, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!("Warning: {message}")))
                                    .await;
                            }
                            PsrpRecord::Information { record, .. } => {
                                let tags = record
                                    .tags
                                    .clone()
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(|t| t.to_ascii_uppercase())
                                    .collect::<Vec<_>>();
                                let has_pshost = tags.iter().any(|t| t == "PSHOST");
                                let has_forwarded = tags.iter().any(|t| t == "FORWARDED");

                                match &record.message_data {
                                    ironposh_psrp::InformationMessageData::HostInformationMessage(m) => {
                                        let (text, newline) = format_host_information_message(m);
                                        let _ = terminal_op_tx
                                            .send(TerminalOperation::Write { text, newline })
                                            .await;
                                    }
                                    ironposh_psrp::InformationMessageData::String(s) => {
                                        let prefix = if has_pshost && !has_forwarded {
                                            ""
                                        } else {
                                            "[information] "
                                        };
                                        let _ = terminal_op_tx
                                            .send(TerminalOperation::Print(format!("{prefix}{s}")))
                                            .await;
                                    }
                                    ironposh_psrp::InformationMessageData::Object(v) => {
                                        let prefix = if has_pshost && !has_forwarded {
                                            ""
                                        } else {
                                            "[information] "
                                        };
                                        let _ = terminal_op_tx
                                            .send(TerminalOperation::Print(format!(
                                                "{prefix}{v}"
                                            )))
                                            .await;
                                    }
                                }
                            }
                            PsrpRecord::Progress { record, .. } => {
                                let status = record.status_description.clone().unwrap_or_default();
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!(
                                        "[progress] {}: {} ({}%)",
                                        record.activity, status, record.percent_complete
                                    )))
                                    .await;
                            }
                            PsrpRecord::Unsupported { data_preview, .. } => {
                                let _ = terminal_op_tx
                                    .send(TerminalOperation::Print(format!(
                                        "[unsupported] {data_preview}"
                                    )))
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run simple REPL mode using basic stdin/stdout
pub async fn run_simple_repl(
    client: &mut RemoteAsyncPowershellClient,
    terminal: Terminal,
    mut hostcall_ui_rx: tokio::sync::mpsc::Receiver<TerminalOperation>,
    mut session_event_rx: futures::channel::mpsc::UnboundedReceiver<SessionEvent>,
    repl_control_rx: Receiver<ReplControl>,
) -> anyhow::Result<()> {
    info!("Starting async REPL with unified UI queue");

    wait_for_active_session(&mut session_event_rx).await?;

    // Channels: UI thread -> REPL (user input events)
    let (terminal_request_tx, terminal_request_rx) = tokio::sync::mpsc::channel::<UserInput>(32);
    let (terminal_op_tx, terminal_op_rx) = tokio::sync::mpsc::channel::<TerminalOperation>(32);

    let terminal_op_tx_1 = terminal_op_tx.clone();
    let forward_handle = tokio::spawn(async move {
        while let Some(op) = hostcall_ui_rx.recv().await {
            if terminal_op_tx_1.send(op).await.is_err() {
                warn!("UI operation channel closed, stopping forwarder");
                break;
            }
        }
    });

    info!("Created unified communication channels");
    let ui_handle = run_ui_thread(terminal, terminal_op_rx, terminal_request_tx.clone());

    info!("UI thread and forwarder tasks spawned, starting unified REPL loop");
    let repl_result =
        run_repl_loop(client, terminal_op_tx, terminal_request_rx, repl_control_rx).await;

    info!("REPL loop ending, cleaning up tasks");

    ui_handle.abort();
    forward_handle.abort();

    info!("Unified async REPL completed");
    repl_result
}
