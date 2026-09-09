import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'

// The shared axios instance defaults to `application/json`; for multipart we
// must clear it so the browser sets `multipart/form-data; boundary=…` itself.
const MULTIPART = { headers: { 'Content-Type': null } }

// ── Single-file parse (POST /api/process) ────────────────────────────────────

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

// ── Operations (profile / validate / convert / transform / diff / pipeline) ──

export type CellType = 'empty' | 'integer' | 'float' | 'boolean' | 'string'

export interface ColumnProfile {
  column: string
  index: number
  inferred_type: CellType
  count: number
  nulls: number
  blanks: number
  distinct: number
  distinct_capped: boolean
  min?: string
  max?: string
  mean?: number
  top: { value: string; count: number }[]
  samples: string[]
}

export interface ProfileReport {
  columns: string[]
  total_rows: number
  profile: ColumnProfile[]
}

export type FieldType = 'string' | 'integer' | 'float' | 'number' | 'boolean'

export interface ColumnRule {
  required?: boolean
  unique?: boolean
  type?: FieldType
  regex?: string
  enum?: string[]
  min?: number
  max?: number
  min_length?: number
  max_length?: number
}

export interface ValidationSchema {
  columns: Record<string, ColumnRule>
  allow_extra_columns?: boolean
  max_errors?: number
}

export interface ValidationReport {
  valid: boolean
  total_rows: number
  error_count: number
  errors_truncated: boolean
  missing_columns: string[]
  unexpected_columns: string[]
  errors: {
    row: number
    column: string
    rule: string
    value: string
    message: string
  }[]
}

export type ConvertTarget = 'csv' | 'json' | 'ndjson' | 'xlsx'

export type FilterOp =
  | 'eq' | 'ne' | 'gt' | 'gte' | 'lt' | 'lte'
  | 'in' | 'not_in' | 'matches' | 'is_null' | 'not_null'

export interface Predicate {
  eq?: unknown
  ne?: unknown
  gt?: number
  gte?: number
  lt?: number
  lte?: number
  in?: unknown[]
  not_in?: unknown[]
  matches?: string
  is_null?: boolean
  not_null?: boolean
}

export interface TransformSpec {
  select?: string[]
  drop?: string[]
  rename?: Record<string, string>
  cast?: Record<string, 'integer' | 'float' | 'string' | 'boolean'>
  filter?: Record<string, Predicate>
  offset?: number
  limit?: number
}

export interface TransformResult {
  columns: string[]
  data: (string | number | boolean | null)[][]
  stats: ParseStats
  matched_rows: number
  timings: Timings
}

export interface DiffReport {
  key: string[]
  summary: {
    added: number
    removed: number
    changed: number
    unchanged: number
    columns_added: string[]
    columns_removed: string[]
  }
  truncated: boolean
  added: { key: Record<string, unknown>; row: Record<string, unknown> }[]
  removed: { key: Record<string, unknown>; row: Record<string, unknown> }[]
  changed: {
    key: Record<string, unknown>
    changes: Record<string, { from: unknown; to: unknown }>
  }[]
}

export type PipelineStep =
  | { op: 'validate'; schema: ValidationSchema; fail_on_error?: boolean }
  | { op: 'transform'; spec: TransformSpec }
  | { op: 'profile' }
  | { op: 'convert'; to: ConvertTarget; out_delimiter?: string }

export interface Pipeline {
  steps: PipelineStep[]
}

// ── Batch jobs (/api/v1/jobs) ───────────────────────────────────────────────

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed'
export type JobKind = 'sync' | 'batch'
export type JobOrigin = 'api_key' | 'oauth_client' | 'admin' | 'service'

export interface FileResult {
  file: string
  rows: number
  columns: number
  elapsed_ms: number
  status: 'ok' | 'error'
  error: string | null
  timings?: Timings
  output?: string | null
}

export type Operation =
  | 'parse' | 'profile' | 'validate' | 'convert' | 'transform' | 'pipeline' | 'batch'

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
  search?: string
}

function qs(options?: Record<string, unknown> | ProcessOptions): string {
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

/** Read a filename out of a Content-Disposition header, with a fallback. */
function dispositionName(header: unknown, fallback: string): string {
  const m = typeof header === 'string' && header.match(/filename="?([^"]+)"?/)
  return m ? m[1] : fallback
}

export const filesApi = {
  /** POST /api/process — synchronous single-file parse. */
  process(file: File, options?: ProcessOptions): Promise<ParsedFile> {
    const form = new FormData()
    form.append('file', file)
    return axios
      .post<ParsedFile>(`${endpoints.files.process}${qs(options)}`, form, MULTIPART)
      .then((r) => r.data)
  },

  /** POST /api/process/profile — per-column stats over the whole file. */
  profile(file: File, options?: ProcessOptions): Promise<{ report: ProfileReport; stats: ParseStats; timings: Timings }> {
    const form = new FormData()
    form.append('file', file)
    return axios
      .post(`${endpoints.files.profile}${qs(options)}`, form, MULTIPART)
      .then((r) => r.data)
  },

  /** POST /api/process/validate — check rows against a column schema. */
  validate(
    file: File,
    schema: ValidationSchema,
    options?: ProcessOptions,
  ): Promise<{ report: ValidationReport; stats: ParseStats; timings: Timings }> {
    const form = new FormData()
    form.append('schema', JSON.stringify(schema))
    form.append('file', file)
    return axios
      .post(`${endpoints.files.validate}${qs(options)}`, form, MULTIPART)
      .then((r) => r.data)
  },

  /** POST /api/process/convert — download the file in another format. */
  async convert(file: File, to: ConvertTarget, options?: ProcessOptions): Promise<void> {
    const form = new FormData()
    form.append('file', file)
    const res = await axios.post(`${endpoints.files.convert}${qs({ ...options, to })}`, form, {
      ...MULTIPART,
      responseType: 'blob',
    })
    const stem = file.name.replace(/\.[^.]+$/, '')
    saveBlob(res.data, dispositionName(res.headers['content-disposition'], `${stem}.${to}`))
  },

  /** POST /api/process/transform — returns JSON, or downloads a file when `to` is set. */
  async transform(
    file: File,
    spec: TransformSpec,
    opts?: { to?: ConvertTarget } & ProcessOptions,
  ): Promise<TransformResult | void> {
    const form = new FormData()
    form.append('spec', JSON.stringify(spec))
    form.append('file', file)
    const { to, ...rest } = opts ?? {}
    if (to) {
      const res = await axios.post(`${endpoints.files.transform}${qs({ ...rest, to })}`, form, {
        ...MULTIPART,
        responseType: 'blob',
      })
      const stem = file.name.replace(/\.[^.]+$/, '')
      saveBlob(res.data, dispositionName(res.headers['content-disposition'], `${stem}.${to}`))
      return
    }
    return axios
      .post<TransformResult>(`${endpoints.files.transform}${qs(rest)}`, form, MULTIPART)
      .then((r) => r.data)
  },

  /** POST /api/process/diff — compare two files on a key column. */
  diff(a: File, b: File, key: string[], options?: ProcessOptions): Promise<{ report: DiffReport; timings: Timings }> {
    const form = new FormData()
    form.append('a', a)
    form.append('b', b)
    return axios
      .post(`${endpoints.files.diff}${qs({ ...options, key: key.join(',') })}`, form, MULTIPART)
      .then((r) => r.data)
  },

  /** POST /api/process/pipeline — run ordered steps; JSON or file download. */
  async pipeline(
    file: File,
    pipeline: Pipeline,
    options?: ProcessOptions,
  ): Promise<{ kind: 'data'; body: TransformResult & { steps: unknown[] } } | { kind: 'file' } | { kind: 'rejected'; steps: unknown[] }> {
    const form = new FormData()
    form.append('pipeline', JSON.stringify(pipeline))
    form.append('file', file)
    const res = await axios.post(`${endpoints.files.pipeline}${qs(options)}`, form, {
      ...MULTIPART,
      responseType: 'blob',
      validateStatus: (s) => s === 200 || s === 422,
    })
    const ct = String(res.headers['content-type'] ?? '')
    if (ct.includes('application/json')) {
      const parsed = JSON.parse(await res.data.text())
      if (res.status === 422) return { kind: 'rejected', steps: parsed.steps ?? [] }
      return { kind: 'data', body: parsed }
    }
    const stem = file.name.replace(/\.[^.]+$/, '')
    saveBlob(res.data, dispositionName(res.headers['content-disposition'], `${stem}.out`))
    return { kind: 'file' }
  },

  /** POST /api/generate/xlsx — JSON rows → downloadable workbook. */
  async generateXlsx(rows: unknown[], opts?: { columns?: string[]; sheet_name?: string }): Promise<void> {
    const res = await axios.post(
      endpoints.files.generateXlsx,
      { rows, columns: opts?.columns, options: opts?.sheet_name ? { sheet_name: opts.sheet_name } : undefined },
      { responseType: 'blob' },
    )
    saveBlob(res.data, dispositionName(res.headers['content-disposition'], `${opts?.sheet_name ?? 'export'}.xlsx`))
  },

  listJobs(params: ListJobsParams = {}): Promise<JobsPage> {
    return axios.get<JobsPage>(endpoints.files.jobs, { params }).then((r) => r.data)
  },

  getJob(id: string): Promise<Job> {
    return axios.get<Job>(endpoints.files.jobDetail(id)).then((r) => r.data)
  },

  /** GET /api/jobs/:id/results — files a batch job generated (needs an api key
   *  scope, so this is best-effort in the session-only admin UI). */
  jobResults(id: string): Promise<{ job_id: string; results: JobResultFile[] }> {
    return axios.get(endpoints.files.jobResults(id)).then((r) => r.data)
  },

  async downloadJobResult(id: string, name: string): Promise<void> {
    const res = await axios.get(endpoints.files.jobResultFile(id, name), { responseType: 'blob' })
    saveBlob(res.data, name)
  },

  /** POST /api/v1/jobs — upload files and start a batch over them. */
  createBatch(
    files: File[],
    output?: { to: ConvertTarget; transform?: TransformSpec },
  ): Promise<{ job_id: string }> {
    const form = new FormData()
    files.forEach((f) => form.append('file', f))
    if (output) form.append('output', JSON.stringify(output))
    return axios
      .post<{ job_id: string }>(endpoints.files.jobs, form, MULTIPART)
      .then((r) => r.data)
  },
}
