//! Zero-dependency SHA-256, RFC 2104 HMAC-SHA256, and strict `<script>` probe tag extraction.

use std::fmt::Write as _;

use jacquard_ports::Failure;

use crate::wrapper::PROBE_ID_PREFIX;

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Computes the SHA-256 digest of `data`.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((data.len() + 9).div_ceil(64) * 64);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        let mut sched = [0u32; 64];
        for (slot, chunk) in sched[..16].iter_mut().zip(block.as_chunks::<4>().0) {
            *slot = u32::from_be_bytes(*chunk);
        }
        for i in 16..64 {
            let s0 = sched[i - 15].rotate_right(7)
                ^ sched[i - 15].rotate_right(18)
                ^ (sched[i - 15] >> 3);
            let s1 = sched[i - 2].rotate_right(17)
                ^ sched[i - 2].rotate_right(19)
                ^ (sched[i - 2] >> 10);
            sched[i] = sched[i - 16]
                .wrapping_add(s0)
                .wrapping_add(sched[i - 7])
                .wrapping_add(s1);
        }
        let [
            mut st_a,
            mut st_b,
            mut st_c,
            mut st_d,
            mut st_e,
            mut st_f,
            mut st_g,
            mut st_h,
        ] = state;
        for i in 0..64 {
            let s1 = st_e.rotate_right(6) ^ st_e.rotate_right(11) ^ st_e.rotate_right(25);
            let ch = (st_e & st_f) ^ ((!st_e) & st_g);
            let t1 = st_h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(sched[i]);
            let s0 = st_a.rotate_right(2) ^ st_a.rotate_right(13) ^ st_a.rotate_right(22);
            let maj = (st_a & st_b) ^ (st_a & st_c) ^ (st_b & st_c);
            let t2 = s0.wrapping_add(maj);
            st_h = st_g;
            st_g = st_f;
            st_f = st_e;
            st_e = st_d.wrapping_add(t1);
            st_d = st_c;
            st_c = st_b;
            st_b = st_a;
            st_a = t1.wrapping_add(t2);
        }
        for (dst, add) in state
            .iter_mut()
            .zip([st_a, st_b, st_c, st_d, st_e, st_f, st_g, st_h])
        {
            *dst = dst.wrapping_add(add);
        }
    }
    let mut out = [0u8; 32];
    for (slot, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *slot = word.to_be_bytes();
    }
    out
}

/// Computes RFC 2104 HMAC-SHA256 for arbitrary key slices.
#[must_use]
pub fn hmac_sha256_raw(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut k_block = [0u8; 64];
    if key.len() > 64 {
        let digest = sha256(key);
        k_block[..32].copy_from_slice(&digest);
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + msg.len());
    for b in k_block {
        inner.push(b ^ 0x36);
    }
    inner.extend_from_slice(msg);
    let inner_hash = sha256(&inner);

    let mut outer = Vec::with_capacity(64 + 32);
    for b in k_block {
        outer.push(b ^ 0x5c);
    }
    outer.extend_from_slice(&inner_hash);
    sha256(&outer)
}

/// Formats lowercase hex HMAC-SHA256 for `"<prefix><nonce>:<json>"`.
#[must_use]
pub fn sign_probe_hex(mac_key: &[u8; 32], nonce: &str, json: &str) -> String {
    let msg = format!("{PROBE_ID_PREFIX}{nonce}:{json}");
    hex_encode(&hmac_sha256_raw(mac_key, msg.as_bytes()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Verifies `sig_hex` against HMAC-SHA256(`mac_key`, `msg`) in constant time.
#[must_use]
pub fn verify_hmac_hex(mac_key: &[u8; 32], msg: &[u8], sig_hex: &str) -> bool {
    let Some(got) = decode_hex32(sig_hex) else {
        return false;
    };
    let expected = hmac_sha256_raw(mac_key, msg);
    let mut diff = 0u8;
    for (a, b) in expected.iter().zip(got.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn decode_hex32(hex: &str) -> Option<[u8; 32]> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let hex_nibble = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = hex_nibble(bytes[i * 2])?;
        let lo = hex_nibble(bytes[i * 2 + 1])?;
        *slot = (hi << 4) | lo;
    }
    Some(out)
}

/// Extracts `(sig_hex, json_slice)` for the single nonced `<script>` inside outer `<head>`.
///
/// # Errors
///
/// Returns [`Failure::Failed`] if nonce count, `<head>` placement, strict `<script>`
/// attributes (`type`, `id`, `data-sig`), or closing tag validation fail.
pub fn extract_probe_tag<'a>(dom: &'a str, nonce: &str) -> Result<(String, &'a str), Failure> {
    let found = dom.to_ascii_lowercase().matches(nonce).count();
    if found != 1 {
        return Err(Failure::Failed(format!(
            "probe nonce integrity check failed (found {found} in {} bytes)",
            dom.len()
        )));
    }
    let head_end = dom
        .find("</head>")
        .ok_or_else(|| Failure::Failed("dumped DOM missing </head>".into()))?;
    let expected_id = format!("{PROBE_ID_PREFIX}{nonce}");
    let id_attr = format!("id=\"{expected_id}\"");
    if dom.matches(&id_attr).count() != 1 || dom[..head_end].matches(&id_attr).count() != 1 {
        return Err(Failure::Failed(
            "expected single nonced probe inside outer <head>".into(),
        ));
    }
    if dom[..head_end].matches(PROBE_ID_PREFIX).count() != 1 {
        return Err(Failure::Failed(
            "unexpected duplicate probe prefix in outer <head>".into(),
        ));
    }
    let id_pos = dom[..head_end]
        .find(&id_attr)
        .ok_or_else(|| Failure::Failed("no probe id".into()))?;
    let tag_start = dom[..id_pos]
        .rfind('<')
        .ok_or_else(|| Failure::Failed("malformed script tag".into()))?;
    if dom[tag_start..id_pos].contains('>') || !dom[tag_start..].starts_with("<script ") {
        return Err(Failure::Failed(
            "probe element must be an HTML <script> tag".into(),
        ));
    }
    let tag_end = dom[id_pos..head_end]
        .find('>')
        .map(|rel| id_pos + rel)
        .ok_or_else(|| Failure::Failed("unterminated <script> opening tag".into()))?;
    let attrs_str = &dom[tag_start + 8..tag_end];
    if attrs_str.contains('<') {
        return Err(Failure::Failed("nested '<' in probe tag attributes".into()));
    }
    let sig_hex = parse_exact_probe_attrs(attrs_str.trim(), &expected_id)?;
    let start = tag_end + 1;
    let end = dom[start..head_end]
        .find("</script>")
        .map(|rel| start + rel)
        .ok_or_else(|| Failure::Failed("unterminated probe report".into()))?;
    let json_slice = &dom[start..end];
    if json_slice.contains('<') {
        return Err(Failure::Failed("probe report contains raw markup".into()));
    }
    Ok((sig_hex, json_slice))
}

fn parse_exact_probe_attrs(attrs: &str, expected_id: &str) -> Result<String, Failure> {
    let mut saw_type = false;
    let mut saw_id = false;
    let mut sig: Option<String> = None;
    for tok in attrs.split_ascii_whitespace() {
        if tok == r#"type="application/json""# && !saw_type {
            saw_type = true;
        } else if tok
            .strip_prefix(r#"id=""#)
            .and_then(|s| s.strip_suffix('"'))
            == Some(expected_id)
            && !saw_id
        {
            saw_id = true;
        } else if let Some(val) = tok
            .strip_prefix(r#"data-sig=""#)
            .and_then(|s| s.strip_suffix('"'))
            && sig.is_none()
        {
            sig = Some(val.to_string());
        } else {
            return Err(Failure::Failed(format!(
                "unexpected probe script attribute: {tok}"
            )));
        }
    }
    if !saw_type || !saw_id {
        return Err(Failure::Failed(
            "probe script missing required type or id".into(),
        ));
    }
    sig.ok_or_else(|| Failure::Failed("probe script missing data-sig".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4231_hmac_sha256_vectors_match() {
        let tc1 = hex_encode(&hmac_sha256_raw(&[0x0b; 20], b"Hi There"));
        assert_eq!(
            tc1,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );

        let tc2 = hex_encode(&hmac_sha256_raw(b"Jefe", b"what do ya want for nothing?"));
        assert_eq!(
            tc2,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }
}
