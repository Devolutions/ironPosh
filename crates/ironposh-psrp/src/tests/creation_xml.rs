use crate::{
    HostDefaultData, HostInfo,
    fragmentation::{DefragmentResult, Defragmenter, Fragmenter},
    messages::{
        ApartmentState, ApplicationArguments, InitRunspacePool, PSThreadOptions, SessionCapability,
    },
};

use tracing::info;
use tracing_test::traced_test;
use uuid::Uuid;

#[test]
#[traced_test]
fn test_combined_messages_like_runspace_open() {
    // Test the exact scenario from RunspacePool::open()
    let session_capability = SessionCapability {
        protocol_version: "2.3".to_string(),
        ps_version: "2.0".to_string(),
        serialization_version: "1.1.0.1".to_string(),
        time_zone: Some("UTC".to_string()),
    };

    let init_runspace_pool = InitRunspacePool {
        min_runspaces: 1,
        max_runspaces: 1,
        thread_options: PSThreadOptions::Default,
        apartment_state: ApartmentState::Unknown,
        host_info: HostInfo::builder()
            .host_default_data(
                HostDefaultData::from_crossterm().expect("Failed to get HostDefaultData"),
            )
            .build(),
        application_arguments: ApplicationArguments::default(),
    };

    let runspace_id = Uuid::parse_str("d034652d-126b-e340-b773-cba26459cfa8").unwrap();

    // Fragment both messages together like RunspacePool::open() does
    let mut fragmenter = Fragmenter::new(32768);
    let messages = vec![
        &session_capability as &dyn crate::PsObjectWithType,
        &init_runspace_pool,
    ];
    let fragmented_bytes = fragmenter
        .fragment_multiple(&messages, runspace_id, None)
        .unwrap();

    info!(
        "Combined messages fragmented bytes len: {}",
        fragmented_bytes.len()
    );

    // Concatenate all fragment bytes for defragmenter
    let mut all_bytes = Vec::new();
    for fragment_bytes in &fragmented_bytes {
        all_bytes.extend_from_slice(fragment_bytes);
    }

    // Try to decode it back using defragmenter
    let mut defragmenter = Defragmenter::new();
    let result = defragmenter.defragment(&all_bytes);
    assert!(
        result.is_ok(),
        "Failed to defragment our own combined messages: {:?}",
        result.err()
    );

    match result.unwrap() {
        DefragmentResult::Complete(messages) => {
            info!("Successfully decoded {} messages!", messages.len());
            assert_eq!(
                messages.len(),
                2,
                "Expected 2 messages (SessionCapability + InitRunspacePool), got {}",
                messages.len()
            );
        }
        DefragmentResult::Incomplete => panic!("Combined messages defragmentation incomplete"),
    }

    info!("Combined messages roundtrip successful!");
}
