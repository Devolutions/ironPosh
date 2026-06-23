//! Characterization snapshots: exact CLIXML of host leaf types, captured from
//! the original hand-written ToPs impls. The derive migration must keep these
//! byte-for-byte (these are credential/host-interaction wire bytes with no live
//! test otherwise).
#![cfg(test)]
use super::methods::*;
use super::traits::ToPs;

fn xml<T: ToPs>(v: T) -> String {
    ToPs::to_ps(v)
        .unwrap()
        .to_element_as_root()
        .unwrap()
        .to_xml_string()
        .unwrap()
}

#[test]
fn coordinates_bytes() {
    assert_eq!(
        xml(Coordinates { x: 3, y: 7 }),
        r#"<Obj RefId="0"><TN RefId="0"><T>System.Management.Automation.Host.Coordinates</T><T>System.ValueType</T><T>System.Object</T></TN><MS><I32 N="X">3</I32><I32 N="Y">7</I32><I32 N="x">3</I32><I32 N="y">7</I32></MS></Obj>"#
    );
}

#[test]
fn size_bytes() {
    assert_eq!(
        xml(Size {
            width: 80,
            height: 25
        }),
        r#"<Obj RefId="0"><TN RefId="0"><T>System.Management.Automation.Host.Size</T><T>System.ValueType</T><T>System.Object</T></TN><MS><I32 N="Height">25</I32><I32 N="Width">80</I32><I32 N="height">25</I32><I32 N="width">80</I32></MS></Obj>"#
    );
}

#[test]
fn keyinfo_bytes() {
    assert_eq!(
        xml(KeyInfo {
            virtual_key_code: 65,
            character: 'A',
            control_key_state: 0,
            key_down: true
        }),
        r#"<Obj RefId="0"><TN RefId="0"><T>System.Management.Automation.Host.KeyInfo</T><T>System.ValueType</T><T>System.Object</T></TN><MS><C N="Character">65</C><I32 N="ControlKeyState">0</I32><B N="KeyDown">true</B><I32 N="VirtualKeyCode">65</I32><C N="character">65</C><I32 N="controlKeyState">0</I32><B N="keyDown">true</B><I32 N="virtualKeyCode">65</I32></MS></Obj>"#
    );
}

#[test]
fn pscredential_bytes() {
    assert_eq!(
        xml(PSCredential {
            user_name: "user".into(),
            password: vec![1, 2, 3]
        }),
        r#"<Obj RefId="0"><TN RefId="0"><T>System.Management.Automation.PSCredential</T><T>System.Object</T></TN><MS><SS N="Password">AQID</SS><S N="UserName">user</S><SS N="password">AQID</SS><S N="userName">user</S></MS></Obj>"#
    );
}

// Verify the DERIVED ToPsValue produces byte-identical output to the locked
// snapshots above (the hand-written ToPs path).
#[test]
fn derived_matches_snapshots() {
    use ironposh_psrp::ps_value::ToPsValue;
    let dx = |v: ironposh_psrp::ps_value::PsValue| {
        v.to_element_as_root().unwrap().to_xml_string().unwrap()
    };
    assert_eq!(
        dx(Coordinates { x: 3, y: 7 }.to_ps_value()),
        xml(Coordinates { x: 3, y: 7 })
    );
    assert_eq!(
        dx(Size {
            width: 80,
            height: 25
        }
        .to_ps_value()),
        xml(Size {
            width: 80,
            height: 25
        })
    );
    let k = KeyInfo {
        virtual_key_code: 65,
        character: 'A',
        control_key_state: 0,
        key_down: true,
    };
    assert_eq!(dx(k.clone().to_ps_value()), xml(k));
    let c = PSCredential {
        user_name: "user".into(),
        password: vec![1, 2, 3],
    };
    assert_eq!(dx(c.clone().to_ps_value()), xml(c));
}
