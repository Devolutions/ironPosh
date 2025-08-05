pub mod deserialize;
mod init_runspace_pool;
mod pipeline_input;
pub mod serialize;
mod session_capability;

use std::{collections::HashMap, hash::Hash};

pub use init_runspace_pool::*;
pub use session_capability::*;

use crate::MessageType;

pub trait PSMessage: Into<PsObject> {
    fn message_type(&self) -> MessageType;
}

/// One PS “primitive” or nested object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PsValue {
    Str(String),     // <S>
    Bool(bool),      // <B>
    I32(i32),        // <I32>
    U32(u32),        // <U32>
    I64(i64),        // <I64>
    Guid(String),    // <G>
    Nil,             // <Nil/>
    Bytes(Vec<u8>),  // <BA>
    Version(String), // <Version>
    Object(PsObject), // <Obj> … </Obj>
                     // Extend as needed...
}

impl Hash for PsValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            PsValue::Str(s) => s.hash(state),
            PsValue::Bool(b) => b.hash(state),
            PsValue::I32(i) => i.hash(state),
            PsValue::U32(u) => u.hash(state),
            PsValue::I64(i) => i.hash(state),
            PsValue::Guid(g) => g.hash(state),
            PsValue::Nil => ().hash(state),
            PsValue::Bytes(b) => b.hash(state),
            PsValue::Version(v) => v.hash(state),
            PsValue::Object(o) => o.to_element().to_string().hash(state), // recursive
        }
    }
}

/// A property wrapper that carries the `N=` and `RefId=` attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsProperty {
    pub name: Option<String>, //  N="..."
    pub ref_id: Option<u32>,  //  RefId="..."
    pub value: PsValue,       //  actual payload
}

/// A full <Obj>.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PsObject {
    pub ref_id: Option<u32>,             // <Obj RefId="...">
    pub type_names: Option<Vec<String>>, // <TN><T>...</T></TN>
    pub tn_ref: Option<u32>,             // <TNRef RefId="..."/>
    pub props: Vec<PsProperty>,          // <Props>  🔸 optional helper bag
    pub ms: Vec<PsProperty>,             // <MS>     🔸 “member set”
    pub lst: Vec<PsProperty>,            // <LST>    🔸 list / array
    pub dct: HashMap<PsValue, PsValue>,  // <DCT>    🔸 dictionary
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        Destination, Fragmenter, MessageType, PowerShellRemotingMessage,
        messages::init_runspace_pool::{
            ApartmentState, HostInfo, InitRunspacePool, PSThreadOptions,
        },
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_creation_xml() {
        // Test the creation of SessionCapability and InitRunspacePool messages
        // similar to the Python open method

        // Create SessionCapability message (like in Python open method)
        let session_capability = SessionCapability {
            protocol_version: "2.2".to_string(),
            ps_version: "2.0".to_string(),
            serialization_version: "1.1.0.1".to_string(),
            time_zone: "AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL".to_string(),
        };

        // Create InitRunspacePool message (like in Python open method)
        let init_runspace_pool = InitRunspacePool {
            min_runspaces: 1,
            max_runspaces: 1,
            thread_options: PSThreadOptions::Default,
            apartment_state: ApartmentState::MTA,
            host_info: None,
            application_arguments: HashMap::new(),
        };

        // Generate UUIDs for the messages
        let rpid = Uuid::new_v4();
        let pid = Uuid::new_v4();

        // Convert to PowerShell remoting messages
        let session_capability_msg = PowerShellRemotingMessage::new(
            Destination::Server,
            MessageType::SessionCapability,
            rpid,
            Some(pid),
            &session_capability.into(),
        );

        let init_runspace_pool_msg = PowerShellRemotingMessage::new(
            Destination::Server,
            MessageType::InitRunspacepool,
            rpid,
            Some(pid),
            &init_runspace_pool.into(),
        );

        // Create fragmenter and fragment the messages (like Python fragmenter.fragment_multiple)
        let mut fragmenter = Fragmenter::new(32768); // 32KB default fragment size
        let messages = vec![session_capability_msg, init_runspace_pool_msg];
        let request_groups = fragmenter.fragment_multiple(&messages);

        // Flatten all fragments for this demo
        let all_fragments: Vec<&crate::fragment::Fragment> = request_groups
            .iter()
            .flat_map(|group| group.iter())
            .collect();
        println!(
            "Generated {} fragments in {} request groups",
            all_fragments.len(),
            request_groups.len()
        );

        // Pack fragments into wire format
        let mut packed_fragments = Vec::new();
        for fragment in &all_fragments {
            packed_fragments.push(fragment.pack());
        }

        // Verify we have fragments
        assert!(
            !packed_fragments.is_empty(),
            "Should have generated fragments"
        );

        // Test unpacking the first fragment
        if let Some(first_packed) = packed_fragments.first() {
            let (unpacked_fragment, remaining) =
                crate::Fragment::unpack(first_packed).expect("Should be able to unpack fragment");

            assert_eq!(remaining.len(), 0, "Should consume entire fragment");
            assert_eq!(
                unpacked_fragment.data.len(),
                first_packed.len() - 21,
                "Data size should match"
            );

            println!(
                "Successfully unpacked fragment with {} bytes of data",
                unpacked_fragment.data.len()
            );
        }

        // Test round-trip: pack all fragments and verify they can be unpacked
        for (i, packed_fragment) in packed_fragments.iter().enumerate() {
            let (unpacked, remaining) = crate::Fragment::unpack(packed_fragment)
                .expect(&format!("Should unpack fragment {}", i));

            assert_eq!(remaining.len(), 0, "Should consume entire packed fragment");
            assert!(unpacked.data.len() > 0, "Fragment should have data");

            // Verify fragment structure
            if i == 0 {
                assert!(unpacked.start, "First fragment should have start flag");
            }
            if i == packed_fragments.len() - 1 {
                assert!(unpacked.end, "Last fragment should have end flag");
            }
        }

        println!("All fragments successfully packed and unpacked");
    }

    #[test]
    fn test_init_runspace_pool_with_application_arguments() {
        // Test InitRunspacePool with application arguments
        let mut app_args = HashMap::new();
        app_args.insert(
            PsValue::Str("TestKey".to_string()),
            PsValue::Str("TestValue".to_string()),
        );
        app_args.insert(PsValue::Str("NumberKey".to_string()), PsValue::I32(42));

        let init_runspace_pool = InitRunspacePool {
            min_runspaces: 2,
            max_runspaces: 5,
            thread_options: PSThreadOptions::UseNewThread,
            apartment_state: ApartmentState::STA,
            host_info: None,
            application_arguments: app_args,
        };

        // Convert to PsObject and then to XML
        let ps_object: PsObject = init_runspace_pool.into();
        let xml_element = ps_object.to_element();
        let xml_string = xml_element.to_string();

        println!(
            "InitRunspacePool with application arguments XML:\n{}",
            xml_string
        );

        // Verify the XML contains expected elements
        assert!(xml_string.contains(r#"N="MinRunspaces""#));
        assert!(xml_string.contains(r#"N="MaxRunspaces""#));
        assert!(xml_string.contains(r#"N="PSThreadOptions""#));
        assert!(xml_string.contains(r#"N="ApartmentState""#));
        assert!(xml_string.contains(r#"N="ApplicationArguments""#));
        assert!(
            xml_string.contains("<DCT>"),
            "Should contain dictionary for application arguments"
        );
    }

    #[test]
    fn test_fragmenter_defragmentation() {
        // Create a large message that will be fragmented
        let init_runspace_pool = InitRunspacePool::default();
        let rpid = Uuid::new_v4();
        let pid = Uuid::new_v4();

        let message = PowerShellRemotingMessage::new(
            Destination::Server,
            MessageType::InitRunspacepool,
            rpid,
            Some(pid),
            &init_runspace_pool.into(),
        );

        // Use a small fragment size to force fragmentation
        let mut fragmenter = Fragmenter::new(100);
        let fragments = fragmenter.fragment(&message);

        println!("Message fragmented into {} pieces", fragments.len());

        // Pack all fragments
        let mut packed_data = Vec::new();
        for fragment in &fragments {
            packed_data.extend_from_slice(&fragment.pack());
        }

        // Unpack all fragments and verify we can reconstruct the original
        let mut cursor = 0;
        let mut unpacked_fragments = Vec::new();

        while cursor < packed_data.len() {
            let (fragment, remaining) =
                crate::Fragment::unpack(&packed_data[cursor..]).expect("Should unpack fragment");
            cursor = packed_data.len() - remaining.len();
            unpacked_fragments.push(fragment);
        }

        assert_eq!(
            fragments.len(),
            unpacked_fragments.len(),
            "Should unpack all fragments"
        );

        // Verify fragment properties match
        for (original, unpacked) in fragments.iter().zip(unpacked_fragments.iter()) {
            assert_eq!(original.object_id, unpacked.object_id);
            assert_eq!(original.fragment_id, unpacked.fragment_id);
            assert_eq!(original.start, unpacked.start);
            assert_eq!(original.end, unpacked.end);
            assert_eq!(original.data, unpacked.data);
        }

        println!("Fragmentation and defragmentation test passed");
    }
}
