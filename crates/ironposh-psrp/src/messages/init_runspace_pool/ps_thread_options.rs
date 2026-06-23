use ironposh_macros::PsEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Management.Automation.Runspaces.PSThreadOptions",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum PSThreadOptions {
    Default = 0,
    UseNewThread = 1,
    ReuseThread = 2,
    UseCurrentThread = 3,
}
