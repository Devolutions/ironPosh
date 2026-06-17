use ironposh_macros::{PsDeserialize, PsEnum, PsSerialize};

/// ProgressRecordType (MS-PSRP §2.2.3.21), serialized as an enum `<Obj>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Management.Automation.ProgressRecordType",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum ProgressRecordType {
    Processing = 0,
    Completed = 1,
}

/// PROGRESS_RECORD message (MS-PSRP §2.2.2.25).
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = ProgressRecord)]
pub struct ProgressRecord {
    #[ps(name = "Activity")]
    pub activity: String,
    #[ps(name = "ActivityId")]
    pub activity_id: i32,
    #[builder(default)]
    #[ps(name = "StatusDescription")]
    pub status_description: Option<String>,
    #[builder(default)]
    #[ps(name = "CurrentOperation")]
    pub current_operation: Option<String>,
    #[builder(default, setter(transform = |x: Option<i32>| x.filter(|&v| v >= 0)))]
    #[ps(name = "ParentActivityId")]
    pub parent_activity_id: Option<i32>,
    #[builder(default, setter(transform = |x: i32| if (-1..=100).contains(&x) { x } else { -1 }))]
    #[ps(name = "PercentComplete")]
    pub percent_complete: i32,
    #[builder(default = ProgressRecordType::Processing)]
    #[ps(name = "Type")]
    pub progress_type: ProgressRecordType,
    #[builder(default)]
    #[ps(name = "SecondsRemaining")]
    pub seconds_remaining: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps_value::{ComplexObject, PsObjectWithType};

    #[test]
    fn test_progress_record_basic() {
        let record = ProgressRecord::builder()
            .activity("Activity Name".to_string())
            .activity_id(4)
            .status_description(Some("Good".to_string()))
            .current_operation(Some("Downloading file.txt".to_string()))
            .parent_activity_id(Some(-1)) // Will be filtered out by builder transform
            .percent_complete(20)
            .progress_type(ProgressRecordType::Processing)
            .seconds_remaining(Some(30))
            .build();

        let complex_obj = ComplexObject::from(record);
        let roundtrip = ProgressRecord::try_from(complex_obj).unwrap();

        let expected = ProgressRecord::builder()
            .activity("Activity Name".to_string())
            .activity_id(4)
            .status_description(Some("Good".to_string()))
            .current_operation(Some("Downloading file.txt".to_string()))
            .parent_activity_id(None)
            .percent_complete(20)
            .progress_type(ProgressRecordType::Processing)
            .seconds_remaining(Some(30))
            .build();

        assert_eq!(expected, roundtrip);
    }

    #[test]
    fn test_progress_record_completed() {
        let record = ProgressRecord::builder()
            .activity("Completed Task".to_string())
            .activity_id(1)
            .percent_complete(100)
            .progress_type(ProgressRecordType::Completed)
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = ProgressRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_message_type() {
        let record = ProgressRecord::builder()
            .activity("Test".to_string())
            .activity_id(0)
            .build();

        assert_eq!(record.message_type().value(), 0x00041010);
    }

    #[test]
    fn test_percent_complete_bounds() {
        let record = ProgressRecord::builder()
            .activity("Test".to_string())
            .activity_id(0)
            .percent_complete(50)
            .build();
        assert_eq!(record.percent_complete, 50);

        let record = ProgressRecord::builder()
            .activity("Test".to_string())
            .activity_id(0)
            .percent_complete(150) // Will be transformed to -1
            .build();
        assert_eq!(record.percent_complete, -1);
    }
}
