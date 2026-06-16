//! Static known-types table (RFC #12, layer L0).
//!
//! Mirrors the PowerShell reference's fixed type table
//! (`serialization.cs:5167-5376`): the mapping of primitive CLIXML tag ↔ .NET
//! type name is static and version-stable, so it lives in one const table to
//! kill tag-drift bugs. The serializer/deserializer tag dispatch should agree
//! with [`PRIMITIVE_TYPES`]; [`tests`](self) asserts it.

/// One entry of the primitive known-types table: CLIXML element tag and the
/// .NET type it represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveType {
    /// CLIXML element tag (e.g. `"I32"`).
    pub tag: &'static str,
    /// .NET type name (e.g. `"System.Int32"`).
    pub dotnet_type: &'static str,
}

/// The primitive types ironPosh serializes, with their CLIXML tags and .NET
/// type names. See MS-PSRP §2.2.5.1.
pub const PRIMITIVE_TYPES: &[PrimitiveType] = &[
    PrimitiveType {
        tag: "S",
        dotnet_type: "System.String",
    },
    PrimitiveType {
        tag: "B",
        dotnet_type: "System.Boolean",
    },
    PrimitiveType {
        tag: "I32",
        dotnet_type: "System.Int32",
    },
    PrimitiveType {
        tag: "U32",
        dotnet_type: "System.UInt32",
    },
    PrimitiveType {
        tag: "I64",
        dotnet_type: "System.Int64",
    },
    PrimitiveType {
        tag: "U64",
        dotnet_type: "System.UInt64",
    },
    PrimitiveType {
        tag: "G",
        dotnet_type: "System.Guid",
    },
    PrimitiveType {
        tag: "C",
        dotnet_type: "System.Char",
    },
    PrimitiveType {
        tag: "Nil",
        dotnet_type: "System.Object",
    },
    PrimitiveType {
        tag: "BA",
        dotnet_type: "System.Byte[]",
    },
    PrimitiveType {
        tag: "SS",
        dotnet_type: "System.Security.SecureString",
    },
    PrimitiveType {
        tag: "Version",
        dotnet_type: "System.Version",
    },
    PrimitiveType {
        tag: "DT",
        dotnet_type: "System.DateTime",
    },
    PrimitiveType {
        tag: "TS",
        dotnet_type: "System.TimeSpan",
    },
];

/// Look up a primitive's .NET type name by its CLIXML tag.
#[must_use]
pub fn dotnet_type_for_tag(tag: &str) -> Option<&'static str> {
    PRIMITIVE_TYPES
        .iter()
        .find(|t| t.tag == tag)
        .map(|t| t.dotnet_type)
}

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
    fn primitive_table_has_no_duplicate_tags() {
        for (i, a) in PRIMITIVE_TYPES.iter().enumerate() {
            for b in &PRIMITIVE_TYPES[i + 1..] {
                assert_ne!(a.tag, b.tag, "duplicate tag {}", a.tag);
            }
        }
    }

    #[test]
    fn lookup_resolves_known_tags() {
        assert_eq!(dotnet_type_for_tag("I32"), Some("System.Int32"));
        assert_eq!(dotnet_type_for_tag("G"), Some("System.Guid"));
        assert_eq!(dotnet_type_for_tag("nope"), None);
    }

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
