use ironposh_macros::PsEnum;

/// MS-PSRP RemoteStreamOptions enum, serialized as a full enum `<Obj>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Management.Automation.RemoteStreamOptions",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum RemoteStreamOptions {
    #[default]
    None = 0,
    AddInvocationInfo = 1,
}
