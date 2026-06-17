use ironposh_macros::PsEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Threading.ApartmentState",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum ApartmentState {
    STA = 0,
    MTA = 1,
    Unknown = 2,
}
