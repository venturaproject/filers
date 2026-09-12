//! Opt-in, best-effort PII redaction for `POST /api/pdf/extract`'s text
//! pipeline — pattern-matching, not a compliance guarantee.
//!
//! This is deliberately **opt-in** (`schema.redact_pii`, default `false`),
//! not a silent default: the endpoint's whole point is reading a document's
//! real values back out, and those values can legitimately *be* the fields
//! being redacted (extract "email" as a field and a blanket email regex
//! would erase exactly what the caller asked for). Turn it on when the
//! surrounding document carries PII the caller does not need — a customer's
//! address block above a table of line items, say.
//!
//! Unlike [`super::strip_metadata`] (which touches opaque image bytes and can
//! only make a parsing mistake, never a wrong redaction), this operates on
//! meaningful text and *will* have false positives and false negatives — an
//! ID number in an unexpected format, a price that happens to look like a
//! phone number. Treat it as a way to reduce exposure, not eliminate it.

use std::sync::LazyLock;

use regex::Regex;

/// `(pattern, placeholder)`, checked in order. Compiled once per process.
static PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Email address.
        (
            Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("valid regex"),
            "[EMAIL]",
        ),
        // IBAN (ES + most EU: 2-letter country, 2 check digits, then the BBAN
        // in 4-character groups).
        (
            Regex::new(r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]{4}){2,7}\b").expect("valid regex"),
            "[IBAN]",
        ),
        // Spanish DNI (8 digits + control letter) and NIE (X/Y/Z + 7 digits +
        // control letter).
        (
            Regex::new(r"(?i)\b(?:\d{8}|[XYZ]\d{7})[A-Z]\b").expect("valid regex"),
            "[ID]",
        ),
        // Card number: 4 groups of 4 digits, optionally separated.
        (
            Regex::new(r"\b(?:\d{4}[ -]?){3}\d{4}\b").expect("valid regex"),
            "[CARD]",
        ),
        // Explicit international phone (a leading `+` is unambiguous — never
        // matches a plain price or quantity).
        (
            Regex::new(r"\+\d{1,3}[ ]?\d{6,14}\b").expect("valid regex"),
            "[PHONE]",
        ),
        // Spanish mobile/landline: 9 digits starting 6/7/8/9, optionally
        // grouped 3-3-3 — narrow enough to not catch a price or an order
        // number written with commas/decimals.
        (
            Regex::new(r"\b[6789]\d{2}[ .-]?\d{3}[ .-]?\d{3}\b").expect("valid regex"),
            "[PHONE]",
        ),
    ]
});

/// Replace recognised PII with a category placeholder. Returns the redacted
/// text and how many replacements were made (surfaced back to the caller in
/// `ExtractResult` so they can sanity-check it actually did something).
pub fn redact(text: &str) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0;
    for (pattern, placeholder) in PATTERNS.iter() {
        let mut n = 0;
        out = pattern
            .replace_all(&out, |_: &regex::Captures| {
                n += 1;
                *placeholder
            })
            .into_owned();
        count += n;
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_an_email() {
        let (out, n) = redact("Contacto: compras@tradivel.example para dudas.");
        assert_eq!(n, 1);
        assert!(out.contains("[EMAIL]"));
        assert!(!out.contains("compras@"));
    }

    #[test]
    fn redacts_a_spanish_dni_and_nie() {
        let (out, n) = redact("Titular: DNI 12345678Z, apoderado NIE X1234567L.");
        assert_eq!(n, 2);
        assert!(!out.contains("12345678Z"));
        assert!(!out.contains("X1234567L"));
    }

    #[test]
    fn redacts_an_iban() {
        let (out, n) = redact("Transferencia a ES91 2100 0418 4502 0005 1332.");
        assert_eq!(n, 1);
        assert!(out.contains("[IBAN]"));
    }

    #[test]
    fn redacts_a_card_number() {
        let (out, n) = redact("Tarjeta 4111 1111 1111 1111 usada en el pago.");
        assert_eq!(n, 1);
        assert!(out.contains("[CARD]"));
    }

    #[test]
    fn redacts_phone_numbers_plain_and_international() {
        let (out, n) = redact("Llamar al 612345678 o al +34 612 345 678.");
        assert_eq!(n, 2);
        assert!(!out.contains("612345678"));
    }

    #[test]
    fn leaves_prices_and_quantities_alone() {
        let text = "12 cajas de tornillos M6, a 3,50 euros la caja. Total 42,00 euros.";
        let (out, n) = redact(text);
        assert_eq!(n, 0);
        assert_eq!(out, text);
    }

    #[test]
    fn is_a_noop_on_plain_text() {
        let text = "Nota de pedido - Almacen Central. Gracias, un saludo.";
        let (out, n) = redact(text);
        assert_eq!(n, 0);
        assert_eq!(out, text);
    }
}
