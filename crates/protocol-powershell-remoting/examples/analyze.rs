use base64::Engine;
use protocol_powershell_remoting::{PowerShellFragment, PsObject, PsValue};
use std::env;
use std::io::{self, Read, Write};
use xml::parser::XmlDeserialize;

fn print_usage() {
    eprintln!("PowerShell Remoting Protocol Analyzer");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {} <base64_encoded_message>", env::args().next().unwrap_or_else(|| "analyze".to_string()));
    eprintln!("  echo '<base64_encoded_message>' | {}", env::args().next().unwrap_or_else(|| "analyze".to_string()));
    eprintln!();
    eprintln!("Description:");
    eprintln!("  Parses and displays PowerShell remoting protocol messages in a human-readable format.");
    eprintln!("  Input should be a base64-encoded PowerShell remoting message.");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} 'AAAAAAAAAAEAAAAAAAAAAAMAAADKAgAAAAIAAQDQ...'", env::args().next().unwrap_or_else(|| "analyze".to_string()));
    eprintln!("  cat message.txt | {}", env::args().next().unwrap_or_else(|| "analyze".to_string()));
}

fn get_input() -> Result<String, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        if args[1] == "--help" || args[1] == "-h" {
            print_usage();
            std::process::exit(0);
        }
        // Use command line argument
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
    println!("{:^80}", title);
    println!("{}", "=".repeat(80));
}

fn print_section(title: &str) {
    println!("\n{}", "-".repeat(60));
    println!("{}", title);
    println!("{}", "-".repeat(60));
}

fn format_ps_object(ps_object: &PsObject, indent: usize) -> String {
    let indent_str = " ".repeat(indent);
    let mut result = format!("{}PsObject {{\n", indent_str);
    
    if let Some(ref_id) = ps_object.ref_id {
        result.push_str(&format!("{}  RefId: {},\n", indent_str, ref_id));
    }
    
    if let Some(ref type_names) = ps_object.type_names {
        result.push_str(&format!("{}  TypeNames: {:?},\n", indent_str, type_names));
    }
    
    if let Some(tn_ref) = ps_object.tn_ref {
        result.push_str(&format!("{}  TNRef: {},\n", indent_str, tn_ref));
    }
    
    if !ps_object.props.is_empty() {
        result.push_str(&format!("{}  Properties: [\n", indent_str));
        for prop in &ps_object.props {
            result.push_str(&format!("{}    {}: {},\n", indent_str, 
                prop.name.as_deref().unwrap_or("(unnamed)"), 
                format_ps_value(&prop.value, indent + 4)));
        }
        result.push_str(&format!("{}  ],\n", indent_str));
    }
    
    if !ps_object.ms.is_empty() {
        result.push_str(&format!("{}  MemberSet: [\n", indent_str));
        for prop in &ps_object.ms {
            result.push_str(&format!("{}    {}: {},\n", indent_str, 
                prop.name.as_deref().unwrap_or("(unnamed)"), 
                format_ps_value(&prop.value, indent + 4)));
        }
        result.push_str(&format!("{}  ],\n", indent_str));
    }
    
    if !ps_object.lst.is_empty() {
        result.push_str(&format!("{}  List: [\n", indent_str));
        for (i, prop) in ps_object.lst.iter().enumerate() {
            result.push_str(&format!("{}    [{}]: {},\n", indent_str, i, 
                format_ps_value(&prop.value, indent + 4)));
        }
        result.push_str(&format!("{}  ],\n", indent_str));
    }
    
    if !ps_object.dct.is_empty() {
        result.push_str(&format!("{}  Dictionary: {{\n", indent_str));
        for (key, value) in &ps_object.dct {
            result.push_str(&format!("{}    {}: {},\n", indent_str, 
                format_ps_value(key, indent + 4), 
                format_ps_value(value, indent + 4)));
        }
        result.push_str(&format!("{}  }},\n", indent_str));
    }
    
    result.push_str(&format!("{}}}", indent_str));
    result
}

fn format_ps_value(ps_value: &PsValue, indent: usize) -> String {
    match ps_value {
        PsValue::Str(s) => format!("\"{}\"", s),
        PsValue::Bool(b) => b.to_string(),
        PsValue::I32(i) => i.to_string(),
        PsValue::U32(u) => u.to_string(),
        PsValue::I64(i) => i.to_string(),
        PsValue::Guid(g) => format!("Guid({})", g),
        PsValue::Nil => "Nil".to_string(),
        PsValue::Bytes(b) => format!("Bytes({:?})", b),
        PsValue::Version(v) => format!("Version({})", v),
        PsValue::Object(obj) => format_ps_object(obj, indent),
    }
}

fn analyze_message(base64_message: &str) -> Result<(), Box<dyn std::error::Error>> {
    print_separator("POWERSHELL REMOTING PROTOCOL ANALYZER");
    
    // Decode base64
    print_section("1. Decoding Base64 Message");
    let engine = base64::engine::general_purpose::STANDARD;
    let message = engine.decode(base64_message.trim())?;
    println!("✓ Successfully decoded {} bytes from base64", message.len());
    
    // Parse PowerShell Fragment
    print_section("2. Parsing PowerShell Fragment");
    let message_slice = message.as_slice();
    let cursor = &mut std::io::Cursor::new(message_slice);
    let fragmented = PowerShellFragment::parse(cursor)?;
    
    println!("✓ Object ID: {}", fragmented.object_id);
    println!("✓ Fragment ID: {}", fragmented.fragment_id);
    println!("✓ Start of fragment: {}", fragmented.start_of_fragment);
    println!("✓ End of fragment: {}", fragmented.end_of_fragment);
    println!("✓ Blob size: {} bytes", fragmented.blob.len());
    
    // Parse PowerShell Remoting Message
    print_section("3. Parsing PowerShell Remoting Message");
    let mut cursor = std::io::Cursor::new(fragmented.blob);
    let pwsh_remoting_message = protocol_powershell_remoting::PowerShellRemotingMessage::parse(&mut cursor)?;
    
    println!("✓ Message Type: {:?}", pwsh_remoting_message.message_type);
    println!("✓ Destination: {:?}", pwsh_remoting_message.destination);
    println!("✓ Runspace Pool ID: {:02x?}", pwsh_remoting_message.rpid);
    println!("✓ Pipeline ID: {:02x?}", pwsh_remoting_message.pid);
    println!("✓ Data size: {} bytes", pwsh_remoting_message.data.len());
    
    // Parse XML Data
    print_section("4. Parsing XML Data");
    let parsed_string_data = match str::from_utf8(&pwsh_remoting_message.data) {
        Ok(s) => s,
        Err(e) => {
            println!("⚠ Warning: Data is not valid UTF-8: {}", e);
            println!("Raw data (first 100 bytes): {:?}", 
                &pwsh_remoting_message.data[..std::cmp::min(100, pwsh_remoting_message.data.len())]);
            return Ok(());
        }
    };
    
    println!("✓ Successfully decoded UTF-8 string ({} characters)", parsed_string_data.len());
    
    if parsed_string_data.len() < 1000 {
        println!("Raw XML Data:");
        println!("{}", parsed_string_data);
    } else {
        println!("Raw XML Data (first 500 characters):");
        println!("{}", &parsed_string_data[..500]);
        println!("... (truncated, {} total characters)", parsed_string_data.len());
    }
    
    // Parse XML
    print_section("5. Parsing XML Structure");
    let xml_representation = match xml::parser::parse(parsed_string_data) {
        Ok(xml) => xml,
        Err(e) => {
            println!("✗ Failed to parse XML: {}", e);
            return Ok(());
        }
    };
    
    println!("✓ Successfully parsed XML structure");
    let root_element = xml_representation.root_element();
    println!("✓ Root element: <{}>", root_element.tag_name().name());
    
    // Parse PowerShell Object
    print_section("6. Converting to PowerShell Object");
    let ps_object = match PsObject::from_node(root_element) {
        Ok(obj) => obj,
        Err(e) => {
            println!("✗ Failed to convert to PowerShell object: {}", e);
            println!("Raw XML root element attributes:");
            for attr in root_element.attributes() {
                println!("  {}: {}", attr.name(), attr.value());
            }
            return Ok(());
        }
    };
    
    println!("✓ Successfully converted to PowerShell object");
    
    // Display formatted PowerShell Object
    print_section("7. Formatted PowerShell Object");
    println!("{}", format_ps_object(&ps_object, 0));
    
    print_separator("ANALYSIS COMPLETE");
    println!("✓ Successfully analyzed PowerShell remoting message");
    
    Ok(())
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    // If command line argument provided, analyze once and exit
    if args.len() > 1 {
        match get_input() {
            Ok(input) => {
                match analyze_message(&input) {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        eprintln!("\n✗ Error analyzing message: {}", e);
                        eprintln!("\nTip: Make sure the input is a valid base64-encoded PowerShell remoting message.");
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("✗ Error reading input: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Interactive mode - continuous loop
        println!("PowerShell Remoting Protocol Analyzer - Interactive Mode");
        println!("========================================================");
        println!();
        println!("Enter base64-encoded PowerShell remoting messages to analyze.");
        println!("Type 'exit' or 'quit' to end the session.");
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
                    
                    match analyze_message(&input) {
                        Ok(()) => {
                            println!("\n{}", "=".repeat(80));
                            println!("Ready for next message...");
                        },
                        Err(e) => {
                            eprintln!("\n✗ Error analyzing message: {}", e);
                            eprintln!("Tip: Make sure the input is a valid base64-encoded PowerShell remoting message.");
                            println!("\nTry again or type 'exit' to quit.");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Error reading input: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
}
