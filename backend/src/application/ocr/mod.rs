//! `POST /api/ocr` — text extraction from an image via a vision-capable LLM
//! behind an OpenAI-compatible `/chat/completions` endpoint (NVIDIA NIM by
//! default: <https://integrate.api.nvidia.com/v1>, model
//! `meta/llama-3.2-11b-vision-instruct`; any other host speaking the same
//! shape works too).
//!
//! This is a genuine OCR path, unlike `pdf::text` (which only reads a PDF's
//! text-showing operators and yields nothing on a scanned page): the image is
//! base64-inlined into the request and the model transcribes what it sees.
//! The upstream API key lives only on this server — a caller authenticates
//! against *this* API's own scopes, never against NVIDIA's.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::errors::{AppError, AppResult};

const DEFAULT_PROMPT: &str = "Transcribe every piece of text visible in this image, verbatim and in \
     reading order. Output only the transcribed text, with no commentary, no \
     markdown fences and no added punctuation.";

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OcrResult {
    pub model: String,
    /// The transcribed text.
    pub text: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Sniff the handful of image formats a vision model accepts, by magic bytes
/// — never trust a client-supplied `Content-Type`. `None` for anything else
/// (including a PDF: rasterising pages is out of scope here).
pub fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

#[derive(Clone)]
pub struct OcrClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
    max_tokens_cap: u32,
}

impl OcrClient {
    /// `None` when `OCR_LLM_API_KEY` is unset — the feature is then simply
    /// absent (the router answers 503 on `/api/ocr`), not half-configured.
    pub fn from_config(config: &Config) -> Option<Self> {
        let api_key = config.ocr_llm_api_key.clone()?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.ocr_llm_timeout_secs))
            .user_agent("filers-ocr/1")
            .build()
            .ok()?;
        Some(Self {
            client,
            base_url: config.ocr_llm_base_url.trim_end_matches('/').to_string(),
            api_key,
            default_model: config.ocr_llm_model.clone(),
            max_tokens_cap: config.ocr_llm_max_tokens,
        })
    }

    /// `image_bytes` must already be a format the sniffed `mime` matches —
    /// see [`sniff_image_mime`]. `prompt` overrides the default OCR
    /// instruction (e.g. "extract just the invoice total").
    pub async fn run(
        &self,
        image_bytes: &[u8],
        mime: &str,
        prompt: Option<&str>,
        max_tokens: Option<u32>,
    ) -> AppResult<OcrResult> {
        use base64::Engine as _;
        // Scrub EXIF/XMP/IPTC before anything leaves this server — a phone
        // photo of a document routinely carries GPS coordinates, the device's
        // serial number and a precise timestamp in its metadata, none of
        // which the model needs to read the text. Best-effort: any parsing
        // uncertainty falls back to the original bytes rather than risk
        // corrupting a real image.
        let scrubbed = strip_metadata(image_bytes, mime);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&scrubbed);
        let data_url = format!("data:{mime};base64,{b64}");
        let prompt = prompt
            .filter(|p| !p.trim().is_empty())
            .unwrap_or(DEFAULT_PROMPT);
        let max_tokens = max_tokens
            .unwrap_or(self.max_tokens_cap)
            .min(self.max_tokens_cap);

        let body = UpstreamRequest {
            model: &self.default_model,
            messages: &[UpstreamMessage {
                role: "user",
                content: vec![
                    UpstreamContent::Text { text: prompt },
                    UpstreamContent::ImageUrl {
                        image_url: UpstreamImageUrl { url: &data_url },
                    },
                ],
            }],
            max_tokens,
            temperature: 0.0,
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("OCR upstream request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let snippet: String = text.chars().take(300).collect();
            return Err(AppError::BadRequest(format!(
                "OCR upstream returned {status}: {snippet}"
            )));
        }

        let parsed: UpstreamResponse = resp.json().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "OCR upstream returned an unexpected body: {e}"
            ))
        })?;

        let choice = parsed.choices.into_iter().next().ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("OCR upstream returned no choices"))
        })?;

        Ok(OcrResult {
            model: parsed.model.unwrap_or_else(|| self.default_model.clone()),
            text: choice
                .message
                .content
                .unwrap_or_default()
                .trim()
                .to_string(),
            finish_reason: choice.finish_reason,
            usage: parsed.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}

/// Drop the metadata segments/chunks a phone or editor tends to embed —
/// EXIF (GPS, device serial, timestamp), XMP, IPTC/Photoshop, free-text
/// comments — before the pixels are handed to a third party. Anything else
/// (dimensions, color profile, the image data itself) passes through
/// untouched. Any parsing uncertainty returns the original bytes: this is a
/// privacy nice-to-have, not a validator, and must never corrupt a real
/// upload.
fn strip_metadata(bytes: &[u8], mime: &str) -> Vec<u8> {
    let stripped = match mime {
        "image/jpeg" => strip_jpeg_metadata(bytes),
        "image/png" => strip_png_metadata(bytes),
        _ => None,
    };
    stripped.unwrap_or_else(|| bytes.to_vec())
}

fn strip_jpeg_metadata(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[0..2]); // SOI
    let mut i = 2;
    loop {
        // JPEG allows 0xFF fill bytes before a marker.
        while bytes.get(i) == Some(&0xFF) && bytes.get(i + 1) == Some(&0xFF) {
            out.push(0xFF);
            i += 1;
        }
        if bytes.get(i) != Some(&0xFF) {
            return None; // expected a marker here — give up, keep the original
        }
        let marker = *bytes.get(i + 1)?;
        // No-length markers (TEM, RSTn) — none of these are expected before
        // the scan starts; bail out to the safe "copy the rest verbatim" path.
        if marker == 0x01 || (0xD0..=0xD9).contains(&marker) {
            out.extend_from_slice(&bytes[i..]);
            return Some(out);
        }
        let len = u16::from_be_bytes([*bytes.get(i + 2)?, *bytes.get(i + 3)?]) as usize;
        if len < 2 {
            return None;
        }
        let segment_end = i.checked_add(2)?.checked_add(len)?;
        if segment_end > bytes.len() {
            return None;
        }
        let payload = &bytes[i + 4..segment_end];
        let drop = match marker {
            // APP1: EXIF or XMP.
            0xE1 => {
                payload.starts_with(b"Exif\0") || payload.starts_with(b"http://ns.adobe.com/xap/")
            }
            // APP13: Photoshop IPTC. COM: free-text comment.
            0xED | 0xFE => true,
            _ => false,
        };
        if !drop {
            out.extend_from_slice(&bytes[i..segment_end]);
        }
        if marker == 0xDA {
            // SOS: the header is copied above; the entropy-coded scan data
            // that follows is opaque to us and copied verbatim to EOF.
            out.extend_from_slice(&bytes[segment_end..]);
            return Some(out);
        }
        i = segment_end;
    }
}

fn strip_png_metadata(bytes: &[u8]) -> Option<Vec<u8>> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if !bytes.starts_with(&SIG) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&SIG);
    let mut i: usize = 8;
    loop {
        if i.checked_add(8)? > bytes.len() {
            return None;
        }
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().ok()?) as usize;
        let ctype = &bytes[i + 4..i + 8];
        let chunk_end = i.checked_add(12)?.checked_add(len)?;
        if chunk_end > bytes.len() {
            return None;
        }
        let drop = matches!(ctype, b"tEXt" | b"zTXt" | b"iTXt" | b"eXIf" | b"tIME");
        if !drop {
            out.extend_from_slice(&bytes[i..chunk_end]);
        }
        if ctype == b"IEND" {
            return Some(out);
        }
        i = chunk_end;
    }
}

// ---- upstream (OpenAI-compatible) request/response shapes ----

#[derive(Serialize)]
struct UpstreamRequest<'a> {
    model: &'a str,
    messages: &'a [UpstreamMessage<'a>],
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize)]
struct UpstreamMessage<'a> {
    role: &'a str,
    content: Vec<UpstreamContent<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UpstreamContent<'a> {
    Text { text: &'a str },
    ImageUrl { image_url: UpstreamImageUrl<'a> },
}

#[derive(Serialize)]
struct UpstreamImageUrl<'a> {
    url: &'a str,
}

#[derive(Debug, Deserialize)]
struct UpstreamResponse {
    model: Option<String>,
    choices: Vec<UpstreamChoice>,
    usage: Option<UpstreamUsage>,
}

#[derive(Debug, Deserialize)]
struct UpstreamChoice {
    message: UpstreamResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpstreamResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpstreamUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_known_image_formats() {
        assert_eq!(
            sniff_image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]),
            Some("image/png")
        );
        assert_eq!(
            sniff_image_mime(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]),
            Some("image/jpeg")
        );
        assert_eq!(sniff_image_mime(b"GIF89a...."), Some("image/gif"));
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBP");
        assert_eq!(sniff_image_mime(&webp), Some("image/webp"));
    }

    #[test]
    fn rejects_non_images() {
        assert_eq!(sniff_image_mime(b"%PDF-1.7"), None);
        assert_eq!(sniff_image_mime(b"not an image"), None);
        assert_eq!(sniff_image_mime(&[]), None);
    }

    #[test]
    fn from_config_is_none_without_an_api_key() {
        let config = crate::bootstrap::test_config(std::env::temp_dir().to_string_lossy());
        assert!(OcrClient::from_config(&config).is_none());
    }

    /// SOI, an APP0/JFIF segment (kept), an APP1/Exif segment (dropped), a
    /// minimal SOS + fake scan data, then EOI.
    fn sample_jpeg_with_exif() -> Vec<u8> {
        let mut b = vec![0xFF, 0xD8]; // SOI
        b.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]); // APP0, len 16
        b.extend_from_slice(b"JFIF\0\x01\x01\x00\x00\x01\x00\x01\x00\x00");
        b.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x15]); // APP1, len 21 (2 + 19-byte payload)
        b.extend_from_slice(b"Exif\0\0GPS-DATA-HERE"); // 19 bytes
        b.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x04, 0x01, 0x02]); // SOS, len 4
        b.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // fake scan data
        b.extend_from_slice(&[0xFF, 0xD9]); // EOI
        b
    }

    #[test]
    fn jpeg_strip_drops_exif_but_keeps_jfif_and_scan_data() {
        let src = sample_jpeg_with_exif();
        let out = strip_jpeg_metadata(&src).unwrap();
        assert!(out.starts_with(&[0xFF, 0xD8]));
        assert!(out.ends_with(&[0xFF, 0xD9]));
        let hay = String::from_utf8_lossy(&out);
        assert!(hay.contains("JFIF"));
        assert!(!hay.contains("Exif"));
        assert!(out.windows(4).any(|w| w == [0x11, 0x22, 0x33, 0x44]));
        assert!(out.len() < src.len());
    }

    #[test]
    fn jpeg_strip_is_a_noop_without_exif() {
        let mut b = vec![0xFF, 0xD8];
        b.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02]);
        b.extend_from_slice(&[0xAA, 0xBB]);
        b.extend_from_slice(&[0xFF, 0xD9]);
        assert_eq!(strip_jpeg_metadata(&b).unwrap(), b);
    }

    #[test]
    fn jpeg_strip_bails_out_on_garbage() {
        assert!(strip_jpeg_metadata(b"not a jpeg").is_none());
        assert!(strip_jpeg_metadata(&[0xFF, 0xD8, 0xFF]).is_none());
    }

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut c = Vec::new();
        c.extend_from_slice(&(data.len() as u32).to_be_bytes());
        c.extend_from_slice(kind);
        c.extend_from_slice(data);
        c.extend_from_slice(&[0, 0, 0, 0]); // fake CRC — we never validate it
        c
    }

    fn sample_png_with_text_chunk() -> Vec<u8> {
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        b.extend_from_slice(&png_chunk(b"IHDR", &[0; 13]));
        b.extend_from_slice(&png_chunk(b"tEXt", b"GPS\0somewhere"));
        b.extend_from_slice(&png_chunk(b"IDAT", &[1, 2, 3, 4]));
        b.extend_from_slice(&png_chunk(b"IEND", &[]));
        b
    }

    #[test]
    fn png_strip_drops_text_chunk_but_keeps_pixels() {
        let src = sample_png_with_text_chunk();
        let out = strip_png_metadata(&src).unwrap();
        assert!(out.starts_with(&[0x89, b'P', b'N', b'G']));
        let hay = String::from_utf8_lossy(&out);
        assert!(!hay.contains("GPS"));
        assert!(out.windows(4).any(|w| w == [1, 2, 3, 4]));
        assert!(out.len() < src.len());
    }

    #[test]
    fn png_strip_bails_out_on_garbage() {
        assert!(strip_png_metadata(b"not a png").is_none());
    }

    #[test]
    fn strip_metadata_falls_back_on_unrecognised_input() {
        // A mime we don't scrub, and outright garbage under a scrubbed mime —
        // both must return the original bytes rather than lose data.
        assert_eq!(strip_metadata(b"whatever", "image/gif"), b"whatever");
        assert_eq!(strip_metadata(b"garbage", "image/jpeg"), b"garbage");
    }
}
