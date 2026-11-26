# Publisher keys for GPG checks (INS-009b)

The installer checks the publishers' OpenPGP signatures with these public keys, built into the
binary (`src/gpg.rs`). A bad signature stops the install; a signature by a key that is not here
gives a warning and the install continues on the SHA-256 and Authenticode checks.

| File | Publisher | Source | Fingerprint |
|---|---|---|---|
| `adoptium.asc` | Eclipse Adoptium (Temurin JDK `.sig` files) | `https://packages.adoptium.net/artifactory/api/gpg/key/public`, same key on `keyserver.ubuntu.com` | `3B04 D753 C905 0D9A 5D34 3F39 843C 48A5 65F8 F04B` |
| `node/*.asc` | Node.js releasers (`SHASUMS256.txt.sig`) | `https://github.com/nodejs/release-keys` (`keys.list` and `keys/<fingerprint>.asc`) | one file per fingerprint, named after it |

Fetched 2026-11-26.

## Updating

Node adds a key when a new person starts cutting releases. When the installer warns
"signed by an unknown key", refresh the Node keys from `nodejs/release-keys` (copy `keys.list`
and every listed `keys/<fingerprint>.asc` into `node/`), add each new file to `NODE_KEYS` in
`src/gpg.rs`, and ship it in the next release. The unit test
`every_node_key_parses_and_matches_its_name` checks every file.
