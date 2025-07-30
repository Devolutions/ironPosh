use protocol_powershell_remoting::{
    Fragmenter, PowerShellRemotingMessage, Destination, MessageType, PsObject,
};
use std::collections::HashMap;
use uuid::Uuid;

fn create_sample_message(content: &str, size_multiplier: usize) -> PowerShellRemotingMessage {
    // Create a sample PsObject with some data
    let ps_object = PsObject {
        ref_id: Some(1),
        type_names: Some(vec!["System.String".to_string()]),
        tn_ref: None,
        props: vec![],
        ms: vec![],
        lst: vec![],
        dct: HashMap::new(),
    };

    PowerShellRemotingMessage::new(
        Destination::Server,
        MessageType::SessionCapability,
        Uuid::new_v4(),
        Uuid::new_v4(),
        &ps_object,
    )
}

fn main() {
    println!("PowerShell Remoting Fragmenter Demo");
    println!("===================================");

    let mut fragmenter = Fragmenter::new(150); // Small fragment size for demo

    // Test 1: Single small message
    println!("\n1. Fragmenting a small message:");
    let small_message = create_sample_message("Small message", 1);
    let fragments = fragmenter.fragment(&small_message);
    println!("   Small message produced {} fragments", fragments.len());
    
    for (i, fragment) in fragments.iter().enumerate() {
        println!("   Fragment {}: object_id={}, fragment_id={}, start={}, end={}, size={}", 
                 i, fragment.object_id, fragment.fragment_id, fragment.start, fragment.end, fragment.data.len());
    }

    // Test 2: Single large message  
    println!("\n2. Fragmenting a large message:");
    let large_message = create_sample_message("Large message", 10);
    let fragments = fragmenter.fragment(&large_message);
    println!("   Large message produced {} fragments", fragments.len());
    
    for (i, fragment) in fragments.iter().enumerate() {
        println!("   Fragment {}: object_id={}, fragment_id={}, start={}, end={}, size={}", 
                 i, fragment.object_id, fragment.fragment_id, fragment.start, fragment.end, fragment.data.len());
    }

    // Test 3: Multiple messages
    println!("\n3. Fragmenting multiple messages:");
    let messages = vec![
        create_sample_message("Message 1", 2),
        create_sample_message("Message 2", 3),
        create_sample_message("Message 3", 1),
    ];
    
    let fragments = fragmenter.fragment_multiple(&messages);
    println!("   {} messages produced {} fragments total", messages.len(), fragments.len());
    
    for (i, fragment) in fragments.iter().enumerate() {
        println!("   Fragment {}: object_id={}, fragment_id={}, start={}, end={}, size={}", 
                 i, fragment.object_id, fragment.fragment_id, fragment.start, fragment.end, fragment.data.len());
    }

    // Test 4: Defragmentation roundtrip
    println!("\n4. Testing fragmentation -> defragmentation roundtrip:");
    
    let original_messages = vec![
        create_sample_message("Test message A", 5),
        create_sample_message("Test message B", 3),
    ];
    
    // Fragment the messages
    let fragments = fragmenter.fragment_multiple(&original_messages);
    println!("   Original {} messages -> {} fragments", original_messages.len(), fragments.len());
    
    // Pack fragments into wire format
    let mut wire_data = Vec::new();
    for fragment in fragments {
        wire_data.extend_from_slice(&fragment.pack());
    }
    println!("   Total wire data size: {} bytes", wire_data.len());
    
    // Defragment back to messages
    match fragmenter.defragment(wire_data, None) {
        Ok((recovered_messages, buffer)) => {
            println!("   Successfully recovered {} messages", recovered_messages.len());
            println!("   Remaining buffer entries: {}", buffer.len());
            
            // Verify messages match
            for (i, (original, recovered)) in original_messages.iter().zip(recovered_messages.iter()).enumerate() {
                let matches = original.destination as u32 == recovered.destination as u32 &&
                             original.message_type.value() == recovered.message_type.value() &&
                             original.rpid == recovered.rpid &&
                             original.pid == recovered.pid;
                println!("   Message {} match: {}", i + 1, matches);
            }
        }
        Err(e) => {
            println!("   Defragmentation failed: {}", e);
        }
    }

    // Test 5: Partial defragmentation (simulating network packets)
    println!("\n5. Testing partial defragmentation:");
    
    let test_message = create_sample_message("Partial test message", 8);
    let fragments = fragmenter.fragment(&test_message);
    println!("   Message fragmented into {} parts", fragments.len());
    
    let mut buffer: Option<HashMap<u64, protocol_powershell_remoting::fragmenter::FragmentBuffer>> = None;
    
    // Send fragments one by one (simulating network packets)
    for (i, fragment) in fragments.iter().enumerate() {
        let wire_data = fragment.pack();
        
        match fragmenter.defragment(wire_data, buffer.take()) {
            Ok((messages, new_buffer)) => {
                buffer = Some(new_buffer);
                println!("   After fragment {}: {} complete messages, {} buffered objects", 
                         i + 1, messages.len(), buffer.as_ref().unwrap().len());
                
                if !messages.is_empty() {
                    println!("   ✓ Message reconstruction complete!");
                    break;
                }
            }
            Err(e) => {
                println!("   Error processing fragment {}: {}", i + 1, e);
                break;
            }
        }
    }

    println!("\nDemo completed!");
}