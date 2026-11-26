//! INS-009b: `OpenPGP` signatures of the toolchain publishers, on top of the SHA-256 and Authenticode
//! checks.
//!
//! * Temurin JDK: Adoptium signs every download; the `.sig` file sits next to the zip
//!   (`signature_link` in the Adoptium API). The whole zip is checked after the download, before
//!   it is extracted.
//! * Node.js: the release manager signs `SHASUMS256.txt` (`SHASUMS256.txt.sig`); the zip's SHA-256
//!   must then be the one in that signed file.
//!
//! The publishers' public keys are built in (`keys/`, sources in `keys/README.md`). Outcome:
//! a valid signature passes; a signature that does not match the data, or that cannot be read,
//! stops the install ([`InstallError::Signature`]); a signature by a key that is not built in
//! (for example a new Node releaser) is reported as [`Verdict::UnknownKey`] so the install can go
//! on with a warning, on the SHA-256 and Authenticode checks.

use std::io::{BufReader, Read};

use pgp::composed::{Deserializable, DetachedSignature, SignedPublicKey};
use pgp::types::KeyDetails;

use crate::InstallError;

/// Whose signature is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Publisher {
    Adoptium,
    Node,
}

impl Publisher {
    /// The name shown in messages.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Adoptium => "Eclipse Adoptium",
            Self::Node => "Node.js",
        }
    }

    /// From the `publisher` stored in a download plan.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "adoptium" => Some(Self::Adoptium),
            "node" => Some(Self::Node),
            _ => None,
        }
    }
}

/// The result of a check that did not fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Signed by this built-in key (primary fingerprint, upper-case hex).
    Valid(String),
    /// Signed by a key that is not built in (issuer, upper-case hex); not checked.
    UnknownKey(String),
}

const ADOPTIUM_KEY: &str = include_str!("../keys/adoptium.asc");

/// Node.js release keys, named by fingerprint (`keys/node/`).
const NODE_KEYS: &[(&str, &str)] = &[
    (
        "108F52B48DB57BB0CC439B2997B01419BD92F80A",
        include_str!("../keys/node/108F52B48DB57BB0CC439B2997B01419BD92F80A.asc"),
    ),
    (
        "114F43EE0176B71C7BC219DD50A3051F888C628D",
        include_str!("../keys/node/114F43EE0176B71C7BC219DD50A3051F888C628D.asc"),
    ),
    (
        "141F07595B7B3FFE74309A937405533BE57C7D57",
        include_str!("../keys/node/141F07595B7B3FFE74309A937405533BE57C7D57.asc"),
    ),
    (
        "1C050899334244A8AF75E53792EF661D867B9DFA",
        include_str!("../keys/node/1C050899334244A8AF75E53792EF661D867B9DFA.asc"),
    ),
    (
        "4ED778F539E3634C779C87C6D7062848A1AB005C",
        include_str!("../keys/node/4ED778F539E3634C779C87C6D7062848A1AB005C.asc"),
    ),
    (
        "56730D5401028683275BD23C23EFEFE93C4CFFFE",
        include_str!("../keys/node/56730D5401028683275BD23C23EFEFE93C4CFFFE.asc"),
    ),
    (
        "5BE8A3F6C8A5C01D106C0AD820B1A390B168D356",
        include_str!("../keys/node/5BE8A3F6C8A5C01D106C0AD820B1A390B168D356.asc"),
    ),
    (
        "61FC681DFB92A079F1685E77973F295594EC4689",
        include_str!("../keys/node/61FC681DFB92A079F1685E77973F295594EC4689.asc"),
    ),
    (
        "655F3B5C1FB3FA8D1A0CA6BDE4A7D232B936D2FD",
        include_str!("../keys/node/655F3B5C1FB3FA8D1A0CA6BDE4A7D232B936D2FD.asc"),
    ),
    (
        "71DCFD284A79C3B38668286BC97EC7A07EDE3FC1",
        include_str!("../keys/node/71DCFD284A79C3B38668286BC97EC7A07EDE3FC1.asc"),
    ),
    (
        "74F12602B6F1C4E913FAA37AD3A89613643B6201",
        include_str!("../keys/node/74F12602B6F1C4E913FAA37AD3A89613643B6201.asc"),
    ),
    (
        "77984A986EBC2AA786BC0F66B01FBB92821C587A",
        include_str!("../keys/node/77984A986EBC2AA786BC0F66B01FBB92821C587A.asc"),
    ),
    (
        "7937DFD2AB06298B2293C3187D33FF9D0246406D",
        include_str!("../keys/node/7937DFD2AB06298B2293C3187D33FF9D0246406D.asc"),
    ),
    (
        "890C08DB8579162FEE0DF9DB8BEAB4DFCF555EF4",
        include_str!("../keys/node/890C08DB8579162FEE0DF9DB8BEAB4DFCF555EF4.asc"),
    ),
    (
        "8FCCA13FEF1D0C2E91008E09770F7A9A5AE15600",
        include_str!("../keys/node/8FCCA13FEF1D0C2E91008E09770F7A9A5AE15600.asc"),
    ),
    (
        "93C7E9E91B49E432C2F75674B0A78B0A6C481CF6",
        include_str!("../keys/node/93C7E9E91B49E432C2F75674B0A78B0A6C481CF6.asc"),
    ),
    (
        "94AE36675C464D64BAFA68DD7434390BDBE9B9C5",
        include_str!("../keys/node/94AE36675C464D64BAFA68DD7434390BDBE9B9C5.asc"),
    ),
    (
        "9554F04D7259F04124DE6B476D5A82AC7E37093B",
        include_str!("../keys/node/9554F04D7259F04124DE6B476D5A82AC7E37093B.asc"),
    ),
    (
        "A363A499291CBBC940DD62E41F10027AF002F8B0",
        include_str!("../keys/node/A363A499291CBBC940DD62E41F10027AF002F8B0.asc"),
    ),
    (
        "A48C2BEE680E841632CD4E44F07496B3EB3C1762",
        include_str!("../keys/node/A48C2BEE680E841632CD4E44F07496B3EB3C1762.asc"),
    ),
    (
        "B9AE9905FFD7803F25714661B63B535A4C206CA9",
        include_str!("../keys/node/B9AE9905FFD7803F25714661B63B535A4C206CA9.asc"),
    ),
    (
        "B9E2F5981AA6E0CD28160D9FF13993A75599653C",
        include_str!("../keys/node/B9E2F5981AA6E0CD28160D9FF13993A75599653C.asc"),
    ),
    (
        "C0D6248439F1D5604AAFFB4021D900FFDB233756",
        include_str!("../keys/node/C0D6248439F1D5604AAFFB4021D900FFDB233756.asc"),
    ),
    (
        "C4F0DFFF4E8C1A8236409D08E73BC641CC11F4C8",
        include_str!("../keys/node/C4F0DFFF4E8C1A8236409D08E73BC641CC11F4C8.asc"),
    ),
    (
        "C82FA3AE1CBEDC6BE46B9360C43CEC45C17AB93C",
        include_str!("../keys/node/C82FA3AE1CBEDC6BE46B9360C43CEC45C17AB93C.asc"),
    ),
    (
        "CC68F5A3106FF448322E48ED27F5E38D5B0A215F",
        include_str!("../keys/node/CC68F5A3106FF448322E48ED27F5E38D5B0A215F.asc"),
    ),
    (
        "DD792F5973C6DE52C432CBDAC77ABFA00DDBF2B7",
        include_str!("../keys/node/DD792F5973C6DE52C432CBDAC77ABFA00DDBF2B7.asc"),
    ),
    (
        "DD8F2338BAE7501E3DD5AC78C273792F7D83545D",
        include_str!("../keys/node/DD8F2338BAE7501E3DD5AC78C273792F7D83545D.asc"),
    ),
    (
        "FD3A5288F042B6850C66B31F09FE44734EB7990E",
        include_str!("../keys/node/FD3A5288F042B6850C66B31F09FE44734EB7990E.asc"),
    ),
];

fn keys(p: Publisher) -> Result<Vec<SignedPublicKey>, InstallError> {
    let parse = |armored: &str| {
        SignedPublicKey::from_string(armored)
            .map(|(k, _)| k)
            .map_err(|e| {
                InstallError::Signature(format!("built-in {} key unreadable: {e}", p.name()))
            })
    };
    match p {
        Publisher::Adoptium => Ok(vec![parse(ADOPTIUM_KEY)?]),
        Publisher::Node => NODE_KEYS.iter().map(|(_, k)| parse(k)).collect(),
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02X}");
        out
    })
}

fn read_signature(sig: &[u8]) -> Result<DetachedSignature, InstallError> {
    let parsed = if sig.starts_with(b"-----BEGIN") {
        DetachedSignature::from_armor_single(sig).map(|(s, _)| s)
    } else {
        DetachedSignature::from_bytes(sig)
    };
    parsed.map_err(|e| InstallError::Signature(format!("the signature file is not valid: {e}")))
}

fn bad(p: Publisher, e: &dyn std::fmt::Display) -> InstallError {
    InstallError::Signature(format!(
        "the {} signature does not match the download ({e}); it was changed or damaged, so it is not used",
        p.name()
    ))
}

/// Checks `sig` (a detached signature, binary or armored) over `data` against `publisher`'s
/// built-in keys. The data is read once, as a stream (large files are fine).
pub fn verify(publisher: Publisher, data: impl Read, sig: &[u8]) -> Result<Verdict, InstallError> {
    let sig = read_signature(sig)?;
    let s = &sig.signature;
    let fprs: Vec<Vec<u8>> = s
        .issuer_fingerprint()
        .iter()
        .map(|f| f.as_bytes().to_vec())
        .collect();
    let ids: Vec<Vec<u8>> = s
        .issuer_key_id()
        .iter()
        .map(|k| k.as_ref().to_vec())
        .collect();
    let issuer = fprs
        .first()
        .or(ids.first())
        .map_or_else(|| "?".to_string(), |b| hex(b));
    let matches = |fp: &[u8], id: &[u8]| {
        fprs.iter().any(|f| f.as_slice() == fp) || ids.iter().any(|i| i.as_slice() == id)
    };
    for key in keys(publisher)? {
        let primary = hex(key.primary_key.fingerprint().as_bytes());
        // the primary key or one of its subkeys may have made the signature
        if matches(
            key.primary_key.fingerprint().as_bytes(),
            key.primary_key.legacy_key_id().as_ref(),
        ) {
            return s
                .verify(&key.primary_key, BufReader::new(data))
                .map(|()| Verdict::Valid(primary))
                .map_err(|e| bad(publisher, &e));
        }
        for sub in &key.public_subkeys {
            if matches(
                sub.key.fingerprint().as_bytes(),
                sub.key.legacy_key_id().as_ref(),
            ) {
                return s
                    .verify(&sub.key, BufReader::new(data))
                    .map(|()| Verdict::Valid(primary))
                    .map_err(|e| bad(publisher, &e));
            }
        }
    }
    Ok(Verdict::UnknownKey(issuer))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUMS: &[u8] = include_bytes!("../tests/fixtures/gpg/SHASUMS256.txt");
    const SUMS_SIG: &[u8] = include_bytes!("../tests/fixtures/gpg/SHASUMS256.txt.sig");
    const NODE_SIGNER: &str = "5BE8A3F6C8A5C01D106C0AD820B1A390B168D356";

    #[test]
    fn a_real_node_signature_is_valid() {
        assert_eq!(
            verify(Publisher::Node, SUMS, SUMS_SIG).unwrap(),
            Verdict::Valid(NODE_SIGNER.into())
        );
    }

    #[test]
    fn a_changed_file_is_refused() {
        let mut changed = SUMS.to_vec();
        changed[10] ^= 1;
        let e = verify(Publisher::Node, changed.as_slice(), SUMS_SIG).unwrap_err();
        assert!(matches!(e, InstallError::Signature(_)), "{e}");
    }

    #[test]
    fn a_signature_by_another_publisher_is_an_unknown_key() {
        // the Node releaser's key is not Adoptium's
        assert_eq!(
            verify(Publisher::Adoptium, SUMS, SUMS_SIG).unwrap(),
            Verdict::UnknownKey(NODE_SIGNER.into())
        );
    }

    #[test]
    fn garbage_is_not_a_signature() {
        assert!(verify(Publisher::Node, SUMS, b"not a signature").is_err());
    }

    #[test]
    fn every_node_key_parses_and_matches_its_name() {
        for (name, armored) in NODE_KEYS {
            let (k, _) = SignedPublicKey::from_string(armored).unwrap();
            assert_eq!(hex(k.primary_key.fingerprint().as_bytes()), *name);
        }
        let (k, _) = SignedPublicKey::from_string(ADOPTIUM_KEY).unwrap();
        assert_eq!(
            hex(k.primary_key.fingerprint().as_bytes()),
            "3B04D753C9050D9A5D343F39843C48A565F8F04B"
        );
    }
}
