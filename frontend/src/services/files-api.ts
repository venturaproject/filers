import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'

// The shared axios instance defaults to `application/json`; for multipart we
// must clear it so the browser sets `multipart/form-data; boundary=…` itself.
const MULTIPART = { headers: { 'Content-Type': null } }

// ── Single-file parse (POST /api/process) — a sanity-check tool in the admin ──

export interface ParseStats {
  total_rows: number
  returned_rows: number
  columns: number
  elapsed_ms: number
}

export interface ParseError {
  row: number
  message: string
}

/** Where the time went (milliseconds). See the Rust `Timings` struct. */
export interface Timings {
  open_ms: number
  read_ms: number
  convert_ms: number
  parse_ms: number
  /** CPU time consumed during the parse. If `parse_ms` >> `parse_cpu_ms` the
   *  host was CPU-starved — the work is cheap, the wait is contention. */
  parse_cpu_ms: number
  upload_ms: number
  total_ms: number
}

export interface ParsedFile {
  format: string
  columns: string[]
  data: (string | number | boolean | null)[][]
  stats: ParseStats
  errors: ParseError[]
  timings: Timings
}

export interface ProcessOptions {
  sheet?: number
  skip_rows?: number
  has_headers?: boolean
  max_rows?: number
  offset?: number
  delimiter?: string
}

// ── Jobs — the processing history the admin monitors ────────────────────────

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed'
export type JobKind = 'sync' | 'batch'
export type JobOrigin = 'api_key' | 'oauth_client' | 'admin' | 'service'
export type Operation =
  | 'parse' | 'profile' | 'validate' | 'convert' | 'transform' | 'pipeline' | 'batch'
  | 'pdf_info' | 'pdf_text' | 'pdf_forms' | 'pdf_split' | 'pdf_merge' | 'pdf_extract'
  | 'ocr'

export interface FileResult {
  file: string
  rows: number
  columns: number
  elapsed_ms: number
  status: 'ok' | 'error'
  error: string | null
  timings?: Timings
  /** Generated result filename, for batch jobs launched with an `output` spec. */
  output?: string | null
}

export interface JobResultFile {
  name: string
  size_bytes: number
  url: string
}

export interface JobSummary {
  id: string
  status: JobStatus
  kind: JobKind
  operation: Operation
  origin: JobOrigin
  actor: string | null
  label: string | null
  files_total: number
  files_processed: number
  files_failed: number
  total_rows: number
  duration_ms: number | null
  created_at: string
  completed_at: string | null
  error: string | null
}

export interface Job extends JobSummary {
  results: FileResult[]
}

export interface JobStats {
  pending: number
  running: number
  completed: number
  failed: number
  avg_ms: number
}

export interface JobsPage {
  data: JobSummary[]
  current_page: number
  last_page: number
  per_page: number
  total: number
  stats: JobStats
}

export interface ListJobsParams {
  page?: number
  per_page?: number
  status?: string
  origin?: string
  operation?: string
  search?: string
}

function qs(options?: ProcessOptions): string {
  const params = new URLSearchParams()
  Object.entries((options ?? {}) as Record<string, unknown>).forEach(([k, v]) => {
    if (v !== undefined && v !== null && v !== '') params.set(k, String(v))
  })
  const s = params.toString()
  return s ? `?${s}` : ''
}

/** Trigger a browser download for a Blob response. */
function saveBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
}

export const filesApi = {
  /** POST /api/process — synchronous single-file parse (admin sanity check). */
  process(file: File, options?: ProcessOptions): Promise<ParsedFile> {
    const form = new FormData()
    form.append('file', file)
    return axios
      .post<ParsedFile>(`${endpoints.files.process}${qs(options)}`, form, MULTIPART)
      .then((r) => r.data)
  },

  listJobs(params: ListJobsParams = {}): Promise<JobsPage> {
    return axios.get<JobsPage>(endpoints.files.jobs, { params }).then((r) => r.data)
  },

  getJob(id: string): Promise<Job> {
    return axios.get<Job>(endpoints.files.jobDetail(id)).then((r) => r.data)
  },

  /** GET /api/v1/jobs/:id/results — files a batch job generated. */
  jobResults(id: string): Promise<{ job_id: string; results: JobResultFile[] }> {
    return axios.get(endpoints.files.jobResults(id)).then((r) => r.data)
  },

  async downloadJobResult(id: string, name: string): Promise<void> {
    const res = await axios.get(endpoints.files.jobResultFile(id, name), { responseType: 'blob' })
    saveBlob(res.data, name)
  },
}
