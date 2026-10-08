//! Ultra HDR (ISO 21496-1 gain map) export, Adobe/Lightroom-style.
//!
//! An Ultra HDR JPEG is an ordinary SDR base JPEG plus a gain map stored as a
//! second image in an MPF (Multi-Picture Format, CIPA DC-007) APP2 segment,
//! described by XMP in the `hdrgm` namespace (`http://ns.adobe.com/hdr-gain-map/1.0/`)
//! and an ISO 21496-1 APP2 segment. HDR-capable viewers reconstruct per-pixel
//! brightness as `sdr * 2^gain`; SDR viewers ignore the extras and show the
//! base image untouched.
//!
//! File layout (matching Adobe Lightroom's HDR export, big-endian MPF):
//! ```text
//! Primary:   SOI | APP0? | APPn(metadata) | APP1(XMP hdrgm version)
//!            | APP2(MPF) | APP2(ISO 21496-1)
//!            | DQT/SOF/DHT… | SOS…EOI
//! Gain map:  SOI | APP1(XMP hdrgm metadata) | APP2(ISO 21496-1)
//!            | JPEG data (RGB, full resolution)
//! ```
//!
//! Encoding side (this module):
//! 1. [`compute_gain_map`] derives per-channel `log2(hdr / sdr)` from the
//!    scene-referred linear HDR rendition and the tone-mapped SDR base, then
//!    quantises it to 8-bit RGB. The map is signed: mid-grey (~128) means
//!    "no gain", brighter means "brighter in HDR", darker means the opposite.
//! 2. The gain map is JPEG-encoded (RGB, full resolution, like Lightroom).
//! 3. [`assemble`] packs base JPEG + gain-map JPEG into one MPF file with the
//!    XMP/ISO metadata above.

use crate::{ChromaSubsampling, EncodeImage, EncodeMeta, Error, Result, Samples, encode_jpeg};

/// Gain-map encoding parameters (mirrors the XMP `hdrgm` fields, log2 stops,
/// one value per channel).
#[derive(Clone, Debug)]
pub struct GainMapParams {
    /// log2 gain range covered by the map, per channel.
    pub min_log2: [f32; 3],
    pub max_log2: [f32; 3],
    pub gamma: [f32; 3],
}

/// A computed gain map, ready to JPEG-encode.
#[derive(Clone, Debug)]
pub struct GainMap {
    pub width: usize,
    pub height: usize,
    /// Interleaved RGB, 8-bit log-encoded gain per channel. Byte `b` maps to
    /// `min_log2[c] + (b/255) * (max_log2[c] - min_log2[c])` (gamma applied
    /// per the XMP when != 1).
    pub data: Vec<u8>,
    pub params: GainMapParams,
}

fn srgb_decode(v: u8) -> f32 {
    lightcraft_color::transfer::srgb_to_linear(v as f32 / 255.0)
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Clamp for the per-channel gain ratio, in stops.
const GAIN_CLAMP_STOPS: f32 = 8.0;

/// Derive a 3-channel gain map from a linear HDR rendition and the SDR base.
///
/// `hdr` is scene-referred linear RGB (output primaries, may exceed 1.0);
/// `sdr` is the tone-mapped 8-bit sRGB base (RGBA pixels).
/// Both must be `width × height`.
///
/// `highlight_boost` (in stops, 0 = off) synthesises extra HDR headroom for
/// SDR sources: per channel it ramps from 0 at mid grey to the full boost at
/// white. This is what makes SDR sources visibly HDR; for true-HDR sources
/// (raw) the real headroom dominates and the boost just extends the top end.
pub fn compute_gain_map(
    hdr: &[[f32; 3]],
    sdr: &[[u8; 4]],
    width: usize,
    height: usize,
    highlight_boost: f32,
) -> Result<GainMap> {
    let n = width.checked_mul(height).ok_or_else(|| Error::Encode("image too large".into()))?;
    if hdr.len() != n || sdr.len() != n {
        return Err(Error::Encode("HDR/SDR dimension mismatch".into()));
    }
    if n == 0 {
        return Err(Error::Encode("empty image".into()));
    }
    let out_len = n.checked_mul(3).ok_or_else(|| Error::Encode("image too large".into()))?;

    // Full-resolution per-channel log2 gain plane.
    let mut log_gain = vec![[0f32; 3]; n];
    let mut min_g = [f32::INFINITY; 3];
    let mut max_g = [f32::NEG_INFINITY; 3];
    for i in 0..n {
        let s = sdr[i];
        let h = hdr[i];
        for c in 0..3 {
            let sdr_lin = srgb_decode(s[c]);
            // Clamp away NaN/negatives; infinities saturate at the clamp below.
            let hdr_lin = h[c].clamp(0.0, 1e9);
            let g = if sdr_lin < 1e-4 {
                // Near-black channel: gain is undefined; black stays black
                // whatever the gain, so record neutral.
                0.0
            } else {
                (hdr_lin / sdr_lin)
                    .clamp(2f32.powf(-GAIN_CLAMP_STOPS), 2f32.powf(GAIN_CLAMP_STOPS))
                    .log2()
            };
            let mut g = if g.is_finite() { g } else { 0.0 };
            // Synthetic highlight expansion for SDR sources (no-op when 0).
            if highlight_boost > 0.0 {
                g += smoothstep(0.35, 1.0, sdr_lin) * highlight_boost;
            }
            log_gain[i][c] = g;
            min_g[c] = min_g[c].min(g);
            max_g[c] = max_g[c].max(g);
        }
    }

    // Per-channel range covering the data and 0 ("no gain"), with a little
    // padding so the endpoints are not razor-thin. Unlike the Google-style
    // map, the range is signed: mid-grey quantises to ~0 gain.
    let mut min_log2 = [0f32; 3];
    let mut max_log2 = [0f32; 3];
    for c in 0..3 {
        let lo = min_g[c].min(0.0);
        let hi = max_g[c].max(0.0);
        let pad = 0.05 * (hi - lo).max(0.5);
        let (mut lo, mut hi) = (lo - pad, hi + pad);
        if hi - lo < 1e-3 {
            lo = -0.25;
            hi = 0.25;
        }
        min_log2[c] = lo;
        max_log2[c] = hi;
    }

    // Quantise to 8-bit RGB.
    let mut data = vec![0u8; out_len];
    for i in 0..n {
        for c in 0..3 {
            let span = max_log2[c] - min_log2[c];
            let inv = if span > 1e-9 { 1.0 / span } else { 0.0 };
            let f = ((log_gain[i][c] - min_log2[c]) * inv).clamp(0.0, 1.0);
            data[i * 3 + c] = (f * 255.0 + 0.5) as u8;
        }
    }

    Ok(GainMap {
        width,
        height,
        data,
        params: GainMapParams { min_log2, max_log2, gamma: [1.0; 3] },
    })
}

/// JPEG-encode a gain map (RGB, high quality, no metadata).
pub fn encode_gain_map(gm: &GainMap, quality: u8) -> Result<Vec<u8>> {
    let img = EncodeImage::new(gm.width as u32, gm.height as u32, 3, Samples::U8(&gm.data));
    let meta = EncodeMeta { icc: None, exif: None, xmp: None, ppi: None };
    encode_jpeg(&img, quality, ChromaSubsampling::S444, &meta)
}

fn fmt_seq(vals: &[f32; 3]) -> String {
    let mut s = String::new();
    s.push_str("<rdf:Seq>\n");
    for v in vals {
        s.push_str(&format!("     <rdf:li>{v:.6}</rdf:li>\n"));
    }
    s.push_str("    </rdf:Seq>");
    s
}

/// XMP packet for the primary image: minimal, just the `hdrgm` version marker
/// (Lightroom does not put a Container Directory on the primary; the MPF
/// segment alone links the gain map).
fn primary_xmp() -> String {
    "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
     <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"LightCraft\">\n\
     \x20<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
     \x20 <rdf:Description rdf:about=\"\"\n\
     \x20  xmlns:hdrgm=\"http://ns.adobe.com/hdr-gain-map/1.0/\"\n\
     \x20  hdrgm:Version=\"1.0\"/>\n\
     \x20</rdf:RDF>\n\
     </x:xmpmeta>\n\
     <?xpacket end=\"w\"?>"
        .to_string()
}

/// XMP packet for the gain-map (secondary) image: the full `hdrgm` metadata in
/// Adobe's RDF-Seq style. Field values mirror what Lightroom writes
/// (`OffsetSDR`/`OffsetHDR` = 1/64, per-channel min/max/gamma).
fn secondary_xmp(p: &GainMapParams) -> String {
    let cap_max = p.max_log2[0].max(p.max_log2[1]).max(p.max_log2[2]).max(0.0);
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"LightCraft\">\n\
         \x20<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         \x20 <rdf:Description rdf:about=\"\"\n\
         \x20  xmlns:hdrgm=\"http://ns.adobe.com/hdr-gain-map/1.0/\"\n\
         \x20  hdrgm:Version=\"1.0\"\n\
         \x20  hdrgm:BaseRenditionIsHDR=\"False\"\n\
         \x20  hdrgm:HDRCapacityMin=\"0\"\n\
         \x20  hdrgm:HDRCapacityMax=\"{cap:.6}\"\n\
         \x20  hdrgm:OffsetSDR=\"0.015625\"\n\
         \x20  hdrgm:OffsetHDR=\"0.015625\">\n\
         \x20  <hdrgm:GainMapMin>\n\
         \x20   {min_seq}\n\
         \x20  </hdrgm:GainMapMin>\n\
         \x20  <hdrgm:GainMapMax>\n\
         \x20   {max_seq}\n\
         \x20  </hdrgm:GainMapMax>\n\
         \x20  <hdrgm:Gamma>\n\
         \x20   {gamma_seq}\n\
         \x20  </hdrgm:Gamma>\n\
         \x20 </rdf:Description>\n\
         \x20</rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>",
        cap = cap_max,
        min_seq = fmt_seq(&p.min_log2),
        max_seq = fmt_seq(&p.max_log2),
        gamma_seq = fmt_seq(&p.gamma),
    )
}

/// Wrap an XMP packet in an APP1 segment (`FF E1`).
fn xmp_segment(packet: &str) -> Vec<u8> {
    const SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/\x00";
    let mut seg = Vec::with_capacity(4 + SIG.len() + packet.len());
    seg.extend_from_slice(&[0xFF, 0xE1]);
    let len = (SIG.len() + packet.len() + 2) as u16;
    seg.extend_from_slice(&len.to_be_bytes());
    seg.extend_from_slice(SIG);
    seg.extend_from_slice(packet.as_bytes());
    seg
}

/// APP2 ISO 21496-1 segment, version only:
/// namespace `"urn:iso:std:iso:ts:21496:-1\0"` + min_version(2B) + writer_version(2B).
fn iso_segment() -> Vec<u8> {
    const SIG: &[u8] = b"urn:iso:std:iso:ts:21496:-1\x00";
    let mut seg = Vec::with_capacity(4 + SIG.len() + 4);
    seg.extend_from_slice(&[0xFF, 0xE2]);
    let len = (SIG.len() + 4 + 2) as u16;
    seg.extend_from_slice(&len.to_be_bytes());
    seg.extend_from_slice(SIG);
    seg.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    seg
}

/// Build the 86-byte MPF APP2 payload, big-endian TIFF header, Adobe-style.
///
/// MP Entry field order per CIPA DC-007: **attribute, size, data offset**,
/// then the two dependent-image entry numbers. Attributes match Lightroom:
/// primary `0x20030000`, gain map `0x00050000`.
fn build_mpf(primary_len: u32, secondary_len: u32, mpf_file_offset: u32) -> Vec<u8> {
    let mut mpf = Vec::with_capacity(86);
    mpf.extend_from_slice(b"MPF\0");
    mpf.extend_from_slice(b"MM");
    mpf.extend_from_slice(&42u16.to_be_bytes());
    mpf.extend_from_slice(&8u32.to_be_bytes()); // IFD offset
    mpf.extend_from_slice(&3u16.to_be_bytes()); // 3 entries
    // MPFVersionID (0xB000): UNDEFINED x4 = "0100".
    mpf.extend_from_slice(&0xB000u16.to_be_bytes());
    mpf.extend_from_slice(&7u16.to_be_bytes());
    mpf.extend_from_slice(&4u32.to_be_bytes());
    mpf.extend_from_slice(b"0100");
    // NumberOfImages (0xB001): LONG x1 = 2.
    mpf.extend_from_slice(&0xB001u16.to_be_bytes());
    mpf.extend_from_slice(&4u16.to_be_bytes());
    mpf.extend_from_slice(&1u32.to_be_bytes());
    mpf.extend_from_slice(&2u32.to_be_bytes());
    // MPEntry (0xB002): UNDEFINED x32, value = offset of MP entries from TIFF header.
    mpf.extend_from_slice(&0xB002u16.to_be_bytes());
    mpf.extend_from_slice(&7u16.to_be_bytes());
    mpf.extend_from_slice(&32u32.to_be_bytes());
    mpf.extend_from_slice(&50u32.to_be_bytes());
    // Next IFD offset = 0.
    mpf.extend_from_slice(&0u32.to_be_bytes());
    // Entry 0 (primary): attribute, size, data offset, dependents.
    mpf.extend_from_slice(&0x2003_0000u32.to_be_bytes());
    mpf.extend_from_slice(&primary_len.to_be_bytes());
    mpf.extend_from_slice(&0u32.to_be_bytes());
    mpf.extend_from_slice(&[0u8; 4]);
    // Entry 1 (gain map): attribute, size, data offset (rel. TIFF header), dependents.
    mpf.extend_from_slice(&0x0005_0000u32.to_be_bytes());
    mpf.extend_from_slice(&secondary_len.to_be_bytes());
    let gain_off = primary_len.wrapping_sub(mpf_file_offset + 8);
    mpf.extend_from_slice(&gain_off.to_be_bytes());
    mpf.extend_from_slice(&[0u8; 4]);
    debug_assert_eq!(mpf.len(), 86);
    // Wrap in APP2: FF E2 | length BE | payload.
    let mut seg = Vec::with_capacity(90);
    seg.extend_from_slice(&[0xFF, 0xE2, 0x00, 88]);
    seg.extend_from_slice(&mpf);
    seg
}

/// A parsed base JPEG: the APP0 JFIF segment (if any), other APPn metadata
/// segments (Exif, ICC, …), the frame segments (DQT/SOF/DHT/…), and the SOS offset.
struct ParsedBase<'a> {
    app0: Option<&'a [u8]>,
    app_meta: Vec<&'a [u8]>,
    frame: Vec<&'a [u8]>,
    sos_offset: usize,
}

fn sig_at(data: &[u8], at: usize, sig: &[u8]) -> bool {
    data.len() >= at + sig.len() && &data[at..at + sig.len()] == sig
}

fn parse_base(base: &[u8]) -> Result<ParsedBase<'_>> {
    if base.len() < 2 || base[0] != 0xFF || base[1] != 0xD8 {
        return Err(Error::Encode("base is not a JPEG".into()));
    }
    const XMP_SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/\x00";
    const ISO_SIG: &[u8] = b"urn:iso:std:iso:ts:21496:-1\x00";
    const MPF_SIG: &[u8] = b"MPF\0";
    let mut pos = 2usize;
    let mut app0: Option<&[u8]> = None;
    let mut app_meta: Vec<&[u8]> = Vec::new();
    let mut frame: Vec<&[u8]> = Vec::new();
    let mut sos_offset = 0usize;
    while pos + 4 <= base.len() {
        if base[pos] != 0xFF {
            return Err(Error::Encode("corrupt JPEG segment".into()));
        }
        let m = base[pos + 1];
        if m == 0xD9 {
            break;
        }
        if m == 0xDA {
            sos_offset = pos;
            break;
        }
        if (0xD0..=0xD7).contains(&m) {
            pos += 2;
            continue;
        }
        let seg_len = u16::from_be_bytes([base[pos + 2], base[pos + 3]]) as usize;
        let end = pos.checked_add(2 + seg_len).ok_or_else(|| Error::Encode("segment overflow".into()))?;
        if end > base.len() {
            return Err(Error::Encode("segment overruns JPEG".into()));
        }
        let seg = &base[pos..end];
        if m == 0xE0 && app0.is_none() {
            app0 = Some(seg);
        } else if m == 0xE1 && sig_at(base, pos + 4, XMP_SIG) {
            // Drop old XMP; we write our own hdrgm version marker.
        } else if m == 0xE2 && (sig_at(base, pos + 4, MPF_SIG) || sig_at(base, pos + 4, ISO_SIG)) {
            // Drop old MPF/ISO; we write our own.
        } else if (0xE0..=0xEF).contains(&m) {
            app_meta.push(seg);
        } else {
            frame.push(seg);
        }
        pos = end;
    }
    if sos_offset == 0 {
        return Err(Error::Encode("JPEG has no SOS".into()));
    }
    Ok(ParsedBase { app0, app_meta, frame, sos_offset })
}

/// Assemble base JPEG + gain-map JPEG into one Ultra HDR file (Adobe layout).
///
/// `base_xmp` is the base image's existing XMP packet, if any: it is preserved
/// as-is (Lightroom puts no Container Directory on the primary). When there is
/// none, a minimal `hdrgm:Version` packet is written.
pub fn assemble(base_jpeg: &[u8], gain_jpeg: &[u8], params: &GainMapParams, base_xmp: Option<&str>) -> Result<Vec<u8>> {
    if gain_jpeg.len() < 2 || gain_jpeg[0] != 0xFF || gain_jpeg[1] != 0xD8 {
        return Err(Error::Encode("gain map is not a JPEG".into()));
    }
    if base_jpeg.len() > u32::MAX as usize || gain_jpeg.len() > u32::MAX as usize {
        return Err(Error::Encode("image too large for MPF".into()));
    }

    // Secondary image: SOI | APP1(XMP hdrgm metadata) | APP2(ISO) | gain JPEG data.
    let mut secondary = Vec::new();
    secondary.extend_from_slice(&[0xFF, 0xD8]);
    secondary.extend_from_slice(&xmp_segment(&secondary_xmp(params)));
    secondary.extend_from_slice(&iso_segment());
    secondary.extend_from_slice(&gain_jpeg[2..]);

    // Primary image head: SOI | APP0 | APPn metadata | APP1(XMP) | APP2(MPF) |
    // APP2(ISO) | frame segments (DQT/SOF/DHT…). MPF goes before SOS.
    let parsed = parse_base(base_jpeg)?;
    let mut head = Vec::new();
    head.extend_from_slice(&[0xFF, 0xD8]);
    if let Some(a0) = parsed.app0 {
        head.extend_from_slice(a0);
    }
    for s in &parsed.app_meta {
        head.extend_from_slice(s);
    }
    head.extend_from_slice(&xmp_segment(base_xmp.unwrap_or(&primary_xmp())));
    let mpf_file_offset = head.len() as u32;
    let iso_seg = iso_segment();
    let frame_len: usize = parsed.frame.iter().map(|s| s.len()).sum();
    let tail_len = base_jpeg.len() - parsed.sos_offset; // SOS..EOI
    // Primary = everything up to (not incl.) the gain map: head + MPF(90) + ISO + frame + tail.
    let total = head.len() as u64 + 90 + iso_seg.len() as u64 + frame_len as u64 + tail_len as u64;
    let primary_len =
        u32::try_from(total).map_err(|_| Error::Encode("image too large for MPF".into()))?;
    head.extend_from_slice(&build_mpf(primary_len, secondary.len() as u32, mpf_file_offset));
    head.extend_from_slice(&iso_seg);
    for s in &parsed.frame {
        head.extend_from_slice(s);
    }

    let mut out = Vec::with_capacity(head.len() + tail_len + secondary.len());
    out.extend_from_slice(&head);
    out.extend_from_slice(&base_jpeg[parsed.sos_offset..]);
    out.extend_from_slice(&secondary);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_enc(lin: f32) -> u8 {
        (lightcraft_color::transfer::linear_to_srgb(lin.clamp(0.0, 1.0)) * 255.0 + 0.5) as u8
    }

    #[test]
    fn gain_map_flat_image_is_neutral() {
        // HDR == SDR everywhere -> gain ~1 -> log2 ~0 -> near-flat map around mid-grey.
        let w = 16;
        let h = 16;
        let hdr = vec![[0.5f32, 0.5, 0.5]; w * h];
        let px = [srgb_enc(0.5), srgb_enc(0.5), srgb_enc(0.5), 255];
        let sdr = vec![px; w * h];
        let gm = compute_gain_map(&hdr, &sdr, w, h, 0.0).unwrap();
        assert_eq!(gm.data.len(), gm.width * gm.height * 3);
        // Near-flat: all bytes within a small band (sRGB round-trip noise only).
        let (mut lo, mut hi) = (255u8, 0u8);
        for b in gm.data.iter() {
            lo = lo.min(*b);
            hi = hi.max(*b);
        }
        assert!(hi - lo <= 12, "flat image must give a flat map, got {lo}..{hi}");
        let mean: f32 = gm.data.iter().map(|b| *b as f32).sum::<f32>() / gm.data.len() as f32;
        assert!((100.0..=156.0).contains(&mean), "mean {mean} should be near 128");
    }

    #[test]
    fn gain_map_highlight_boost() {
        // One HDR pixel 4x brighter than its SDR counterpart -> ~2 stops of gain.
        let w = 8;
        let h = 8;
        let mut hdr = vec![[0.18f32, 0.18, 0.18]; w * h];
        hdr[0] = [0.72, 0.72, 0.72];
        let px = [srgb_enc(0.18), srgb_enc(0.18), srgb_enc(0.18), 255];
        let sdr = vec![px; w * h];
        let gm = compute_gain_map(&hdr, &sdr, w, h, 0.0).unwrap();
        for c in 0..3 {
            assert!(gm.params.max_log2[c] > 1.8, "max_log2[{c}] = {}", gm.params.max_log2[c]);
            assert!(gm.params.max_log2[c] < 2.4, "max_log2[{c}] = {}", gm.params.max_log2[c]);
            assert!(gm.params.min_log2[c] <= 0.0);
        }
        // The hot pixel's bytes are well above neutral.
        assert!(gm.data[0] > 200 && gm.data[1] > 200 && gm.data[2] > 200);
    }

    #[test]
    fn gain_map_rejects_bad_input() {
        assert!(compute_gain_map(&[], &[], 0, 0, 0.0).is_err());
        assert!(compute_gain_map(&[[1.0, 1.0, 1.0]], &[[0u8; 4]; 2], 1, 1, 0.0).is_err());
    }

    #[test]
    fn highlight_boost_lifts_whites_per_channel() {
        // HDR == SDR everywhere (true gain ~0); boost applies to bright channels only.
        let w = 8;
        let h = 4;
        let white_lin = 1.0f32;
        let grey_lin = 0.18f32;
        let hdr: Vec<[f32; 3]> = (0..w * h)
            .map(|i| {
                let v = if i < w { white_lin } else { grey_lin };
                [v, v, v]
            })
            .collect();
        let white = [255u8, 255, 255, 255];
        let grey = [srgb_enc(grey_lin), srgb_enc(grey_lin), srgb_enc(grey_lin), 255];
        let sdr: Vec<[u8; 4]> = (0..w * h).map(|i| if i < w { white } else { grey }).collect();
        let gm = compute_gain_map(&hdr, &sdr, w, h, 2.0).unwrap();
        for c in 0..3 {
            assert!(gm.params.max_log2[c] > 1.7, "max_log2[{c}] = {}", gm.params.max_log2[c]);
            assert!(gm.params.max_log2[c] < 2.4, "max_log2[{c}] = {}", gm.params.max_log2[c]);
        }
        // White row bytes are bright (top of the fitted range); the grey row
        // has ~0 gain, which sits near the bottom of this range.
        assert!(gm.data[0] > 200);
        let grey_px = w * 3; // first pixel of second row, R channel
        assert!(gm.data[grey_px] < 60, "got {}", gm.data[grey_px]);
    }

    #[test]
    fn xmp_packets() {
        let p = GainMapParams { min_log2: [-0.5, -0.5, -0.5], max_log2: [2.5, 2.5, 2.5], gamma: [1.0, 1.0, 1.0] };
        let prim = primary_xmp();
        assert!(prim.contains("hdrgm:Version=\"1.0\""));
        assert!(!prim.contains("Container:Directory"));
        let sec = secondary_xmp(&p);
        assert!(sec.contains("http://ns.adobe.com/hdr-gain-map/1.0/"));
        assert!(sec.contains("<hdrgm:GainMapMin>"));
        assert!(sec.contains("<rdf:li>-0.500000</rdf:li>"));
        assert!(sec.contains("<rdf:li>2.500000</rdf:li>"));
        assert!(sec.contains("hdrgm:OffsetSDR=\"0.015625\""));
        assert!(sec.contains("hdrgm:OffsetHDR=\"0.015625\""));
        assert!(sec.contains("hdrgm:HDRCapacityMin=\"0\""));
        assert!(sec.contains("hdrgm:HDRCapacityMax=\"2.500000\""));
        assert!(sec.contains("hdrgm:BaseRenditionIsHDR=\"False\""));
        // No Google container directory anywhere.
        assert!(!sec.contains("Container:Directory"));
    }

    #[test]
    fn mpf_is_big_endian_adobe_style() {
        let mpf_seg = build_mpf(1000, 200, 100);
        // APP2 wrapper.
        assert_eq!(&mpf_seg[0..2], &[0xFF, 0xE2]);
        let payload = &mpf_seg[4..];
        assert_eq!(&payload[0..4], b"MPF\0");
        assert_eq!(&payload[4..6], b"MM", "must be big-endian like Lightroom");
        // MP entries: attribute(4) | size(4) | offset(4), big-endian.
        let entries = 4 + 50; // TIFF header + offset of MP entries
        let attr0 = u32::from_be_bytes(payload[entries..entries + 4].try_into().unwrap());
        let attr1 = u32::from_be_bytes(payload[entries + 16..entries + 20].try_into().unwrap());
        assert_eq!(attr0, 0x2003_0000, "primary attribute like Lightroom");
        assert_eq!(attr1, 0x0005_0000, "gain map attribute like Lightroom");
        let size0 = u32::from_be_bytes(payload[entries + 4..entries + 8].try_into().unwrap());
        assert_eq!(size0, 1000);
    }

    #[test]
    fn mpf_assembly_structure() {
        // Minimal fake JPEGs (SOI + EOI only); structure is what we check.
        let base = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xE0, 0x00, 0x04, 0xAA, 0xBB, // APP0 (len 4)
            0xFF, 0xDA, 0x00, 0x02, // SOS (len 2)
            0x11, 0x22, // entropy data
            0xFF, 0xD9, // EOI
        ];
        let gain = vec![0xFF, 0xD8, 0xAA, 0xBB, 0xFF, 0xD9];
        let params =
            GainMapParams { min_log2: [0.0, 0.0, 0.0], max_log2: [1.5, 1.5, 1.5], gamma: [1.0, 1.0, 1.0] };
        let out = assemble(&base, &gain, &params, None).unwrap();
        // SOI, then APP0 preserved right after.
        assert_eq!(&out[0..2], &[0xFF, 0xD8]);
        assert_eq!(&out[2..4], &[0xFF, 0xE0]);
        // Primary XMP carries the hdrgm version, no container directory.
        let out_str = String::from_utf8_lossy(&out);
        assert!(out_str.contains("hdrgm:Version=\"1.0\""));
        assert!(!out_str.contains("Container:Directory"));
        // Gain-map XMP has the RDF-Seq metadata.
        assert!(out_str.contains("<hdrgm:GainMapMin>"));
        assert!(out_str.contains("hdrgm:OffsetSDR=\"0.015625\""));
        // ISO 21496-1 APP2 present (primary).
        assert!(out.windows(28).any(|w| w == b"urn:iso:std:iso:ts:21496:-1\x00"));
        // Order: MPF -> ISO -> SOS.
        let mpf_pos = out.windows(4).position(|w| w == b"MPF\0").unwrap();
        let sos_pos = out.windows(2).position(|w| w == &[0xFF, 0xDA]).unwrap();
        assert!(mpf_pos < sos_pos, "MPF must come before SOS");
        let iso_pos = out.windows(28).position(|w| w == b"urn:iso:std:iso:ts:21496:-1\x00").unwrap();
        assert!(mpf_pos < iso_pos && iso_pos < sos_pos, "order must be MPF -> ISO -> SOS");
        // MPF payload is 86 bytes, big-endian TIFF header.
        assert_eq!(&out[mpf_pos..mpf_pos + 4], b"MPF\0");
        assert_eq!(&out[mpf_pos + 4..mpf_pos + 6], b"MM");
        // Primary size = whole file up to primary EOI.
        let entries = mpf_pos + 4 + 50;
        let attr0 = u32::from_be_bytes(out[entries..entries + 4].try_into().unwrap());
        assert_eq!(attr0, 0x2003_0000);
        let size0 = u32::from_be_bytes(out[entries + 4..entries + 8].try_into().unwrap());
        let primary_eoi = out.windows(2).position(|w| w == &[0xFF, 0xD9]).unwrap() + 2;
        assert_eq!(size0 as usize, primary_eoi);
        // Gain map offset (rel. TIFF header) lands on its SOI.
        let off1 = u32::from_be_bytes(out[entries + 24..entries + 28].try_into().unwrap());
        let tiff_header = mpf_pos + 4;
        assert_eq!(&out[tiff_header + off1 as usize..tiff_header + off1 as usize + 2], &[0xFF, 0xD8]);
        // Gain map carries its own XMP + ISO.
        let size1 = u32::from_be_bytes(out[entries + 20..entries + 24].try_into().unwrap());
        let gain_start = tiff_header + off1 as usize;
        let gain_end = gain_start + size1 as usize;
        assert_eq!(out[gain_end - 2..gain_end], [0xFF, 0xD9]);
        let gain_str = String::from_utf8_lossy(&out[gain_start..gain_end]);
        assert!(gain_str.contains("<hdrgm:GainMapMax>"));
        // Errors.
        assert!(assemble(&[0x00, 0x00], &gain, &params, None).is_err());
        assert!(assemble(&base, &[0x00], &params, None).is_err());
    }

    #[test]
    fn gain_map_jpeg_encodes_rgb() {
        let gm = GainMap {
            width: 4,
            height: 4,
            data: vec![128u8; 48],
            params: GainMapParams { min_log2: [0.0, 0.0, 0.0], max_log2: [2.0, 2.0, 2.0], gamma: [1.0, 1.0, 1.0] },
        };
        let jpeg = encode_gain_map(&gm, 90).unwrap();
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    }
}
