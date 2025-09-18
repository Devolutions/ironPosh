use base64::Engine;
use ironposh_psrp::{
    ComplexObject, DefragmentResult, Defragmenter,
    messages::deserialize::{DeserializationContext, PsXmlDeserialize},
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn print_usage() {
    eprintln!("PowerShell Remoting Protocol Analyzer");
    eprintln!();
    eprintln!("Usage:");
    eprintln!(
        "  {} <base64_encoded_message>",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!(
        "  {} --file <file_path>",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!(
        "  {} --multi <fragment1> <fragment2> ...",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!(
        "  echo '<base64_encoded_message>' | {}",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!();
    eprintln!("Description:");
    eprintln!(
        "  Parses and displays PowerShell remoting protocol messages in a human-readable format."
    );
    eprintln!("  Input should be a base64-encoded PowerShell remoting message or fragment.");
    eprintln!("  Use --multi flag to defragment multiple fragments into complete messages.");
    eprintln!();
    eprintln!("Examples:");
    eprintln!(
        "  {} 'AAAAAAAAAAEAAAAAAAAAAAMAAADKAgAAAAIAAQDQ...'",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!(
        "  {} --file message.txt",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!(
        "  {} --multi 'fragment1_base64' 'fragment2_base64' 'fragment3_base64'",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
    eprintln!(
        "  cat message.txt | {}",
        env::args().next().unwrap_or_else(|| "analyze".to_string())
    );
}

fn get_input() -> Result<String, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        if args[1] == "--help" || args[1] == "-h" {
            print_usage();
            std::process::exit(0);
        }

        // Handle --file flag
        if args[1] == "--file" && args.len() > 2 {
            let file_path = &args[2];
            println!("📁 Reading from file: {file_path}");

            if !Path::new(file_path).exists() {
                return Err(format!("File not found: {file_path}").into());
            }

            let file_content = fs::read_to_string(file_path)?;
            let content = file_content.trim().to_string();

            if content.is_empty() {
                return Err("File is empty".into());
            }

            println!(
                "✅ Successfully read {} characters from file",
                content.len()
            );
            return Ok(content);
        }

        // Use command line argument as base64 message
        Ok(args[1].clone())
    } else {
        // Interactive mode - prompt user for input
        println!("PowerShell Remoting Protocol Analyzer");
        println!("=====================================");
        println!();
        println!("Please paste your base64-encoded PowerShell remoting message:");
        println!("(Press Enter when done, or type 'exit' to quit)");
        println!();
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() {
            return Err("No input provided".into());
        }

        if input.to_lowercase() == "exit" {
            println!("Goodbye!");
            std::process::exit(0);
        }

        Ok(input)
    }
}

fn print_separator(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("{title:^80}");
    println!("{}", "=".repeat(80));
}

fn print_section(title: &str) {
    println!("\n{}", "-".repeat(60));
    println!("{title}");
    println!("{}", "-".repeat(60));
}

fn try_defragment_multiple_messages(
    messages: &[&str],
) -> Result<Vec<ironposh_psrp::PowerShellRemotingMessage>, Box<dyn std::error::Error>> {
    print_section("Multi-Fragment Defragmentation");
    println!(
        "📦 Processing {} fragment(s) for reassembly...",
        messages.len()
    );

    let mut defragmenter = Defragmenter::new();
    let mut completed_messages = Vec::new();
    let engine = base64::engine::general_purpose::STANDARD;

    for (i, msg) in messages.iter().enumerate() {
        let fragment_data = engine.decode(msg.trim())?;
        println!(
            "🔍 Fragment {}: Decoded {} bytes from base64",
            i + 1,
            fragment_data.len()
        );

        match defragmenter.defragment(&fragment_data) {
            Ok(DefragmentResult::Complete(mut msgs)) => {
                if msgs.is_empty() {
                    println!("⏳ Fragment {} processed, no complete messages yet", i + 1);
                } else {
                    println!("✅ Fragment {} completed {} message(s)!", i + 1, msgs.len());
                }
                completed_messages.append(&mut msgs);
            }
            Ok(DefragmentResult::Incomplete) => {
                println!(
                    "⏳ Fragment {} processed, waiting for more fragments to complete message(s)",
                    i + 1
                );
            }
            Err(e) => {
                println!("❌ Error processing fragment {}: {}", i + 1, e);
                return Err(e.into());
            }
        }
    }

    if defragmenter.pending_count() > 0 {
        println!(
            "⚠️  Warning: {} incomplete message(s) still in buffer (may need more fragments)",
            defragmenter.pending_count()
        );
    }

    if completed_messages.is_empty() {
        println!(
            "ℹ️  No complete messages assembled - fragments may be incomplete or out of order"
        );
    } else {
        println!(
            "🎉 Defragmentation successful! Assembled {} complete PowerShell remoting message(s)",
            completed_messages.len()
        );
    }
    Ok(completed_messages)
}

fn analyze_message(base64_message: &str) -> Result<(), Box<dyn std::error::Error>> {
    print_separator("POWERSHELL REMOTING PROTOCOL ANALYZER");

    // Decode base64
    print_section("1. Base64 Decoding");
    let engine = base64::engine::general_purpose::STANDARD;
    let message = engine.decode(base64_message.trim())?;
    println!(
        "✅ Successfully decoded {} bytes from base64 input",
        message.len()
    );

    // Parse PowerShell Fragment
    print_section("2. PowerShell Remoting Message Parsing");
    let message_slice = message.as_slice();
    let mut defragmenter = Defragmenter::new();
    let messages = match defragmenter.defragment(message_slice)? {
        DefragmentResult::Incomplete => {
            println!(
                "⚠️  This appears to be a fragment that requires additional fragments to complete"
            );
            println!(
                "💡 Try using the --multi flag with all fragments to reassemble the complete message"
            );
            return Err("Incomplete message, waiting for more fragments".into());
        }
        DefragmentResult::Complete(power_shell_remoting_messages) => power_shell_remoting_messages,
    };

    println!(
        "🎉 Successfully parsed {} complete PowerShell remoting message(s)",
        messages.len()
    );

    print_section("3. Message Summary");
    for (i, msg) in messages.iter().enumerate() {
        println!("📨 Message {} Details:", i + 1);
        println!("   📋 Type: {:?}", msg.message_type);
        println!("   🎯 Destination: {:?}", msg.destination);
        println!("   🆔 Runspace Pool ID: {}", msg.rpid);
        if let Some(pid) = msg.pid {
            println!("   🔗 Pipeline ID: {pid}");
        }
        println!("   📏 Data Size: {} bytes", msg.data.len());
        println!();
    }

    for (msg_idx, pwsh_remoting_message) in messages.iter().enumerate() {
        if messages.len() > 1 {
            print_separator(&format!("ANALYZING MESSAGE {}", msg_idx + 1));
        }
        print_section("4. Data Extraction & UTF-8 Decoding");
        let parsed_string_data = match str::from_utf8(&pwsh_remoting_message.data) {
            Ok(s) => s,
            Err(e) => {
                println!("⚠️  Warning: Message data is not valid UTF-8: {e}");
                println!(
                    "🔍 Raw binary data (first 100 bytes): {:?}",
                    &pwsh_remoting_message.data
                        [..std::cmp::min(100, pwsh_remoting_message.data.len())]
                );
                println!("💡 This might be binary data or use a different encoding");
                continue;
            }
        };

        println!(
            "✅ Successfully decoded UTF-8 string ({} characters)",
            parsed_string_data.len()
        );

        println!("📄 Complete XML Data:");
        println!("{parsed_string_data}");

        // Parse XML
        print_section("5. XML Structure Analysis");
        let xml_representation = match ironposh_xml::parser::parse(parsed_string_data) {
            Ok(xml) => xml,
            Err(e) => {
                println!("❌ Failed to parse XML structure: {e}");
                println!("💡 The data may not be valid XML or may be corrupted");
                continue;
            }
        };

        println!("✅ Successfully parsed XML structure");
        let root_element = xml_representation.root_element();
        println!("🏷️  Root XML element: <{}>", root_element.tag_name().name());

        // Parse PowerShell Object
        print_section("6. PowerShell Object Conversion");
        let mut context = DeserializationContext::new();
        let ps_object = match ComplexObject::from_node_with_context(root_element, &mut context) {
            Ok(obj) => obj,
            Err(e) => {
                println!("❌ Failed to convert XML to PowerShell object: {e}");
                println!("🔍 Available XML attributes:");
                for attr in root_element.attributes() {
                    println!("   • {}: {}", attr.name(), attr.value());
                }
                println!("💡 The XML structure may not match expected PowerShell object format");
                continue;
            }
        };

        println!("✅ Successfully converted to PowerShell object representation");

        // Display formatted PowerShell Object
        print_section("7. PowerShell Object Details");
        println!("{ps_object}");
    }

    print_separator("ANALYSIS COMPLETE");
    println!("🎉 Successfully analyzed all PowerShell remoting messages!");

    Ok(())
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Check for --multi flag for defragmentation
    if args.len() > 2 && args[1] == "--multi" {
        print_separator("MULTI-FRAGMENT DEFRAGMENTATION MODE");
        let fragments: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();

        match try_defragment_multiple_messages(&fragments) {
            Ok(messages) => {
                if messages.is_empty() {
                    println!(
                        "ℹ️  No complete messages could be assembled from the provided fragments"
                    );
                    println!(
                        "💡 This may indicate fragments are missing, out of order, or corrupted"
                    );
                    std::process::exit(1);
                }

                for (i, message) in messages.iter().enumerate() {
                    print_separator(&format!("DEFRAGMENTED MESSAGE {} SUMMARY", i + 1));
                    println!("📨 Message Type: {:?}", message.message_type);
                    println!("🎯 Destination: {:?}", message.destination);
                    println!("🆔 Runspace Pool ID: {}", message.rpid);
                    if let Some(pid) = message.pid {
                        println!("🔗 Pipeline ID: {pid}");
                    }
                    println!("📏 Data Size: {} bytes", message.data.len());
                    println!("✅ Message successfully reconstructed from fragments!");
                }

                println!(
                    "\n💡 Use single message mode to perform detailed analysis of each message"
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("\n❌ Error during defragmentation: {e}");
                eprintln!("💡 Tips for troubleshooting:");
                eprintln!("   • Ensure all fragments are valid base64-encoded data");
                eprintln!("   • Check that fragments are provided in the correct order");
                eprintln!("   • Verify that no fragments are missing from the sequence");
                eprintln!("   • Make sure fragments belong to the same original message");
                std::process::exit(1);
            }
        }
    }

    // If command line argument provided, analyze once and exit
    if args.len() > 1 {
        match get_input() {
            Ok(input) => match analyze_message(&input) {
                Ok(()) => std::process::exit(0),
                Err(e) => {
                    eprintln!("\n❌ Analysis failed: {e}");
                    eprintln!("💡 Troubleshooting suggestions:");
                    eprintln!("   • Verify the input is valid base64-encoded data");
                    eprintln!(
                        "   • Check if this is a fragment that needs other fragments (try --multi)"
                    );
                    eprintln!("   • Ensure the data represents a PowerShell remoting message");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("❌ Input error: {e}");
                eprintln!("💡 Use --help for usage information");
                std::process::exit(1);
            }
        }
    } else {
        // Interactive mode - continuous loop
        println!("🔍 PowerShell Remoting Protocol Analyzer - Interactive Mode");
        println!("============================================================");
        println!();
        println!("📝 Commands:");
        println!("   • Enter base64-encoded PowerShell remoting messages to analyze");
        println!("   • Type 'multi' to enter multi-fragment defragmentation mode");
        println!("   • Type 'help' for usage information");
        println!("   • Type 'exit' or 'quit' to end the session");
        println!();

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let input = input.trim().to_string();

                    if input.is_empty() {
                        continue;
                    }

                    if input.to_lowercase() == "exit" || input.to_lowercase() == "quit" {
                        println!("Goodbye!");
                        break;
                    }

                    if input == "help" || input == "--help" || input == "-h" {
                        print_usage();
                        continue;
                    }

                    if input == "multi" {
                        println!("🔧 Entering multi-fragment defragmentation mode");
                        println!("📝 Instructions:");
                        println!("   • Enter each fragment as base64-encoded data");
                        println!("   • Type 'done' when all fragments are entered");
                        println!("   • Type 'cancel' to abort and return to main mode");
                        println!();

                        let mut fragments = Vec::new();

                        loop {
                            print!("fragment {}> ", fragments.len() + 1);
                            io::stdout().flush()?;

                            let mut fragment_input = String::new();
                            io::stdin().read_line(&mut fragment_input)?;
                            let fragment_input = fragment_input.trim();

                            if fragment_input.is_empty() {
                                continue;
                            }

                            if fragment_input == "done" {
                                if fragments.is_empty() {
                                    println!(
                                        "⚠️  No fragments entered. Please add at least one fragment or type 'cancel'."
                                    );
                                    continue;
                                }
                                break;
                            }

                            if fragment_input == "cancel" {
                                println!("❌ Multi-fragment mode cancelled");
                                fragments.clear();
                                break;
                            }

                            fragments.push(fragment_input.to_string());
                            println!(
                                "✅ Added fragment {} (base64 length: {} characters)",
                                fragments.len(),
                                fragment_input.len()
                            );
                        }

                        if !fragments.is_empty() {
                            let fragment_refs: Vec<&str> =
                                fragments.iter().map(|s| s.as_str()).collect();
                            match try_defragment_multiple_messages(&fragment_refs) {
                                Ok(messages) => {
                                    if messages.is_empty() {
                                        println!(
                                            "ℹ️  No complete messages assembled from fragments"
                                        );
                                    } else {
                                        for (i, message) in messages.iter().enumerate() {
                                            print_separator(&format!(
                                                "DEFRAGMENTED MESSAGE {}",
                                                i + 1
                                            ));
                                            println!("📨 Message Type: {:?}", message.message_type);
                                            println!("🎯 Destination: {:?}", message.destination);
                                            println!("🆔 Runspace Pool ID: {}", message.rpid);
                                            if let Some(pid) = message.pid {
                                                println!("🔗 Pipeline ID: {pid}");
                                            }
                                            println!("📏 Data Size: {} bytes", message.data.len());
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Defragmentation error: {e}");
                                    eprintln!(
                                        "💡 Check that fragments are valid and in correct order"
                                    );
                                }
                            }
                        }

                        println!("\n{}", "=".repeat(80));
                        println!("🔄 Ready for next message or command...");
                        continue;
                    }

                    match analyze_message(&input) {
                        Ok(()) => {
                            println!("\n{}", "=".repeat(80));
                            println!("🔄 Ready for next message or command...");
                        }
                        Err(e) => {
                            eprintln!("\n❌ Analysis failed: {e}");
                            eprintln!("💡 Troubleshooting tips:");
                            eprintln!("   • Ensure input is valid base64-encoded data");
                            eprintln!(
                                "   • Try 'multi' mode if this is a fragment needing reassembly"
                            );
                            eprintln!("   • Type 'help' for more information");
                            println!("\n🔄 Try again or type 'exit' to quit.");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Input error: {e}");
                    eprintln!("💡 Please try again or restart the application");
                    break;
                }
            }
        }

        Ok(())
    }
}
