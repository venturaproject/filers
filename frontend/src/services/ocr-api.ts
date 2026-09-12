import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'

// The shared axios instance defaults to `application/json`; for multipart we
// must clear it so the browser sets `multipart/form-data; boundary=…` itself.
const MULTIPART = { headers: { 'Content-Type': null } }

export interface AiUsage {
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
}

export interface OcrResult {
  model: string
  text: string
  finish_reason: string | null
  usage: AiUsage | null
}

export interface ExtractSchema {
  instruction?: string
  fields?: string[]
  redact_pii?: boolean
}

export interface ExtractResult {
  items: Record<string, unknown>[]
  model: string
  usage: AiUsage | null
  truncated_input: boolean
  pii_redactions: number
}

/** `{ error: "..." }` — every endpoint's error body shape. */
export interface ApiErrorBody {
  error?: string
}

export const ocrApi = {
  /** POST /api/ocr — image → transcribed text via a vision LLM. */
  ocr(file: File, prompt?: string): Promise<OcrResult> {
    const form = new FormData()
    form.append('file', file)
    if (prompt?.trim()) form.append('prompt', prompt.trim())
    return axios.post<OcrResult>(endpoints.ai.ocr, form, MULTIPART).then((r) => r.data)
  },

  /** POST /api/pdf/extract — a PDF's text → the caller's requested JSON shape. */
  extract(file: File, schema?: ExtractSchema): Promise<ExtractResult> {
    const form = new FormData()
    form.append('file', file)
    const hasSchema =
      schema && (schema.instruction?.trim() || schema.fields?.length || schema.redact_pii)
    if (hasSchema) form.append('schema', JSON.stringify(schema))
    return axios.post<ExtractResult>(endpoints.ai.pdfExtract, form, MULTIPART).then((r) => r.data)
  },
}
