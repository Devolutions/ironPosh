# PowerShell Remoting Protocol Comparison: Success vs Failed

## Status: Critical Differences Identified ❌

The PowerShell remoting initialization messages show significant structural and content differences between success and failed attempts. The failed message is missing critical application metadata that the server requires for proper session establishment.

## Key Differences Summary

| Aspect | Success | Failed | Impact |
|--------|---------|--------|--------|
| **Message 1 (SessionCapability)** | ✅ Identical | ✅ Identical | No issue |
| **Message 2 Size** | 3,781 bytes | 509 bytes | 🔴 87% smaller |
| **ApplicationArguments** | Complex PSVersionTable | `<Nil N="ApplicationArguments"/>` | 🔴 Missing metadata |
| **HostInfo** | Detailed host configuration | Missing entirely | 🔴 No host info |
| **RefId Structure** | Proper object references | Broken references | 🔴 Serialization issues |

## Detailed Analysis

### Message 1: SessionCapability ✅
Both success and failed messages are identical for session capability negotiation:
- Protocol version: 2.3
- PowerShell version: 2.0  
- Serialization version: 1.1.0.1

### Message 2: InitRunspacepool ❌

#### ApplicationArguments Comparison

**Success Message:**
```xml
<Obj N="ApplicationArguments" RefId="3">
  <TN RefId="2">
    <T>System.Management.Automation.PSPrimitiveDictionary</T>
    <T>System.Collections.Hashtable</T>
    <T>System.Object</T>
  </TN>
  <DCT>
    <En>
      <S N="Key">PSVersionTable</S>
      <Obj N="Value" RefId="4">
        <!-- Contains detailed version information -->
        <DCT>
          <En><S N="Key">OS</S><S N="Value">Microsoft Windows 10.0.22631</S></En>
          <En><S N="Key">PSVersion</S><Version N="Value">7.4.10</Version></En>
          <En><S N="Key">PSCompatibleVersions</S>...</En>
          <En><S N="Key">SerializationVersion</S><Version N="Value">1.1.0.1</Version></En>
          <En><S N="Key">PSEdition</S><S N="Value">Core</S></En>
          <En><S N="Key">PSRemotingProtocolVersion</S><Version N="Value">2.3</Version></En>
          <!-- ... more version details -->
        </DCT>
      </Obj>
    </En>
  </DCT>
</Obj>
```

**Failed Message:**
```xml
<Nil N="ApplicationArguments"/>
```

#### HostInfo Comparison

**Success Message:** Contains extensive host configuration data including:
- Console dimensions (120x30 buffer, 3824x2121 max window)
- Color settings (foreground/background)
- Cursor position and window coordinates
- Host name ("PowerShell")
- UI capability flags

**Failed Message:** Missing entirely - no HostInfo object present.

#### Object Reference Structure

**Success:** Uses proper RefId system with sequential numbering (RefId="0" through RefId="24")
**Failed:** Inconsistent RefId usage, objects without proper references

## Root Cause Analysis

The server rejection is caused by missing critical initialization data:

1. **PSVersionTable Missing**: Server cannot determine client PowerShell capabilities, OS version, or supported features
2. **Host Information Absent**: Server cannot configure proper console/UI interaction
3. **Incomplete Object Graph**: Broken serialization references may cause deserialization failures

## Next Steps

To fix the connection issues:

1. **Implement ApplicationArguments**: Generate proper PSVersionTable with:
   - OS version information
   - PowerShell version and edition details  
   - Compatible version arrays
   - Serialization and remoting protocol versions

2. **Add HostInfo Object**: Include console configuration:
   - Buffer and window size settings
   - Color and cursor position defaults
   - Host name and UI capability flags

3. **Fix Object References**: Ensure proper RefId numbering and cross-references in the serialized object graph

4. **Validate Serialization**: Compare byte-level structure with working PowerShell client output

The failed message appears to be a minimal/incomplete initialization that lacks the metadata required for proper PowerShell remoting session establishment.