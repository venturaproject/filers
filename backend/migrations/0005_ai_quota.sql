-- Per-client cap on tokens spent via the LLM-backed endpoints
-- (POST /api/ocr, POST /api/pdf/extract) — independent of the existing
-- page-based monthly_page_quota, since a client can be capped on one, both,
-- or neither.
ALTER TABLE api_clients
    ADD COLUMN IF NOT EXISTS monthly_ai_token_quota BIGINT,
    ADD COLUMN IF NOT EXISTS usage_ai_tokens BIGINT NOT NULL DEFAULT 0;
