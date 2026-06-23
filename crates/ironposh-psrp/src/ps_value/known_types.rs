//! Conventions for PowerShell's primitive CLIXML types (RFC #12, layer L0).

/// Format a [`uuid::Uuid`] the way PowerShell serializes `System.Guid`:
/// uppercase, hyphenated, no braces.
#[must_use]
pub fn format_guid(guid: uuid::Uuid) -> String {
    guid.to_string().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_convention_is_uppercase() {
        let g = uuid::Uuid::nil();
        assert_eq!(format_guid(g), "00000000-0000-0000-0000-000000000000");
        let g = "a1b2c3d4-1111-2222-3333-444455556666"
            .parse::<uuid::Uuid>()
            .unwrap();
        assert_eq!(format_guid(g), "A1B2C3D4-1111-2222-3333-444455556666");
    }
}
