use ironposh_macros::PsEnum;

/// MS-PSRP PipelineResultTypes enum, serialized as a full enum `<Obj>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Management.Automation.Runspaces.PipelineResultTypes",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum PipelineResultTypes {
    #[default]
    None = 0x00,
    Output = 0x01,
    Error = 0x02,
    Warning = 0x04,
    Verbose = 0x08,
    Debug = 0x10,
    All = 0x20,
    Null = 0x40,
}
