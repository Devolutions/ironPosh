use super::PsValue;

pub struct PipelineInput {
    pub data: PsValue,
}

impl PipelineInput {
    pub fn new(data: PsValue) -> Self {
        PipelineInput { data }
    }
}

// impl From<PipelineInput> for PsObject {
//     fn from(input: PipelineInput) -> Self {
//         PsObject {
//             ms: vec![PsProperty {
//                 name: Some("Data".to_string()),
//                 ref_id: None,
//                 value: input.data,
//             }],
//             ..Default::default()
//         }
//     }
// }
