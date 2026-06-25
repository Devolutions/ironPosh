//! Single integration-test binary aggregating every e2e suite.
//!
//! Each module under `tests/e2e/` was previously its own root-level
//! integration test (one linked binary per file); grouping them here keeps
//! one binary and one link step. All tests remain `#[ignore]`d unless a real
//! WinRM server is reachable (see each suite's ignore message).

mod auths;
mod clixml_primitives;
mod command_latency;
mod command_matrix;
mod configuration_name;
mod disconnect_reconnect;
mod native_pty_matrix;
mod pty_cancel_then_next;
mod pty_ctrl_c;
mod pty_delayed_burst;
mod pty_error_stream;
mod pty_idle_ctrl_c;
mod pty_interactive_commands;
mod pty_large_output;
mod pty_mixed_streams;
mod pty_nested_scripts;
mod pty_prompt_ctrl_c;
mod pty_rapid_sequential;
mod pty_readhost_ctrl_c_race;
mod pty_real_server_additional;
mod pty_records;
mod pty_second_while_first_runs;
mod pty_session_cleanup;
mod pty_stress_terminal;
mod pty_tab_completion;
mod pty_terminal_hostcalls_matrix;
mod pty_terminating_error;
mod real_server_feature;
mod reattach;
mod transport_auth_matrix;
