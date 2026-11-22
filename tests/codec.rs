use btrcmpr::{Frame, MAX_BLOCK, compress_with, decode, decompress_with, encode};
fn mixed(n: usize, seed: u32) -> Vec<u8> {
    let mut state = seed;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    let mut v = Vec::with_capacity(n);
    while v.len() < n {
        let x = next();
        let count = (x as usize % 521 + 1).min(n - v.len());
        match x % 6 {
            0 => v.extend((0..count).map(|_| next() as u8)),
            1 => v.resize(v.len() + count, 0x5a),
            2 if !v.is_empty() => {
                let off = next() as usize % v.len() + 1;
                for _ in 0..count {
                    v.push(v[v.len() - off])
                }
            }
            _ => {
                let step = x as usize % 17 + 1;
                let start = v.len();
                v.extend((0..count).map(|j| ((j + start) % step) as u8))
            }
        }
    }
    v
}
#[test]
fn roundtrip_boundaries_and_parallel_archive() {
    for &(n, t) in &[
        (0, 4),
        (1, 4),
        (17, 1),
        (65, 2),
        (8193, 3),
        (131071, 4),
        (MAX_BLOCK + 137, 4),
    ] {
        let v = mixed(n, 47);
        let frame = compress_with(&v, t, encode).unwrap();
        let mut bytes = Vec::new();
        frame.write(&mut bytes).unwrap();
        assert_eq!(bytes.len(), frame.encoded_size());
        let parsed = Frame::read(&bytes).unwrap();
        let mut out: Vec<_> = v.iter().map(|x| !x).collect();
        decompress_with(&parsed, 3, &mut out, decode).unwrap();
        assert_eq!(v, out);
        assert!(Frame::read(&bytes[..bytes.len() - 1]).is_err());
        bytes.push(0);
        assert!(Frame::read(&bytes).is_err());
    }
}
#[test]
fn sixteen_bit_prediction_with_outliers_and_odd_tail() {
    let mut data = Vec::new();
    for i in 0usize..45047 {
        let v = if i % 391 == 0 {
            (i * 32731) as u16
        } else {
            (2000 + i / 100 + i % 37) as u16
        };
        data.extend(v.to_le_bytes())
    }
    data.push(17);
    let adjacent = btrcmpr::encode_preset(&data, btrcmpr::Preset::new(1).unwrap()).unwrap();
    assert_eq!(adjacent[0], 6);
    let selected = encode(&data).unwrap();
    assert!(selected.len() <= adjacent.len());

    {
        // This signal has a repeating row component as well as outliers.
        assert_eq!(selected[0], 63);
        assert!(selected.len() < adjacent.len());
    }
    for frame in [adjacent, selected] {
        let mut out = vec![0xcc; data.len()];
        decode(&frame, &mut out).unwrap();
        assert_eq!(out, data);
        assert!(decode(&frame[..frame.len() - 1], &mut out).is_err());
    }
}
#[test]
fn corrupt_block_is_bounded() {
    let s = mixed(70003, 345);
    let v = encode(&s).unwrap();
    assert_eq!(v[0], 1);
    let mut guard = vec![0x3f; s.len() + 64];
    for i in (0..v.len()).step_by((v.len() / 77).max(1)) {
        assert!(decode(&v[..i], &mut guard[32..32 + s.len()]).is_err());
        let mut changed = v.clone();
        changed[i] ^= 0xe7;
        let _ = decode(&changed, &mut guard[32..32 + s.len()]);
        assert!(
            guard[..32]
                .iter()
                .chain(&guard[32 + s.len()..])
                .all(|&x| x == 0x3f)
        )
    }
}
#[test]
fn presets_roundtrip_and_reject_invalid_levels() {
    use btrcmpr::{Preset, compress_preset, decompress};
    for n in [0, 23, 255] {
        assert!(Preset::new(n).is_err());
    }
    let d = mixed(32781, 7211);
    for level in 1..=22 {
        let f = compress_preset(&d, 3, Preset::new(level).unwrap()).unwrap();
        let mut bytes = vec![];
        f.write(&mut bytes).unwrap();
        let p = Frame::read(&bytes).unwrap();
        let mut out = vec![0xd3; d.len()];
        decompress(&p, 2, &mut out).unwrap();
        assert_eq!(d, out);
    }
}
