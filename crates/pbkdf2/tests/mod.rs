#![cfg(feature = "hmac")]
use hex_literal::hex;
use pbkdf2::pbkdf2_hmac_array as f;
use sha1::Sha1;
use sha2::Sha256;

/// Tests from RFC 6070:
/// https://www.rfc-editor.org/rfc/rfc6070
#[test]
fn rfc6070() {
    assert_eq!(
        f::<Sha1, 20>(b"password", b"salt", 1),
        hex!("0c60c80f961f0e71f3a9b524af6012062fe037a6"),
    );
    assert_eq!(
        f::<Sha1, 20>(b"password", b"salt", 2),
        hex!("ea6c014dc72d6f8ccd1ed92ace1d41f0d8de8957"),
    );
    assert_eq!(
        f::<Sha1, 20>(b"password", b"salt", 4096),
        hex!("4b007901b765489abead49d926f721d065a429c1"),
    );
    // this test passes, but takes a long time to execute
    /*
    assert_eq!(
        f::<Sha1, 20>(b"password", b"salt", 16777216),
        hex!("eefe3d61cd4da4e4e9945b3d6ba2158c2634e984"),
    );
    */
    assert_eq!(
        f::<Sha1, 25>(
            b"passwordPASSWORDpassword",
            b"saltSALTsaltSALTsaltSALTsaltSALTsalt",
            4096
        ),
        hex!("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038"),
    );
    assert_eq!(
        f::<Sha1, 16>(b"pass\0word", b"sa\0lt", 4096),
        hex!("56fa6aa75548099dcc37d7f03425e0c3"),
    );
}

#[test]
fn rfc6070_sha256() {
    // SHA-256 based PBKDF2 test
    assert_eq!(
        f::<Sha256, 32>(b"password", b"salt", 1),
        hex!("120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"),
    );
}
