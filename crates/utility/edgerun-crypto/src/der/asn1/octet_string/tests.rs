use crate::der::asn1::{OctetStringRef, PrintableStringRef};

#[test]
fn octet_string_decode_into() {
    // PrintableString "hi"
    let der = b"\x13\x02\x68\x69";
    let oct = OctetStringRef::new(der).unwrap();

    let res = oct.decode_into::<PrintableStringRef<'_>>().unwrap();
    assert_eq!(AsRef::<str>::as_ref(&res), "hi");
}
