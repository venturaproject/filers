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

export interface ParsedFile {
  format: string
  columns: string[]
  data: (string | number | boolean | null)[][]
  stats: ParseStats
  errors: ParseError[]
}

export interface ProcessOptions {
  sheet?: number
  skip_rows?: number
  has_headers?: boolean
  max_rows?: number
  offset?: number
  delimiter?: string
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
}

export interface JobSummary {
  id: string
  status: JobStatus
  kind: JobKind
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

export const filesApi = {
  /** POST /api/process — synchronous single-file parse. */
  process(file: File, options?: ProcessOptions): Promise<ParsedFile> {
    const form = new FormData()
    form.append('file', file)
    const params = new URLSearchParams()
    Object.entries(options ?? {}).forEach(([k, v]) => {
      if (v !== undefined && v !== null) params.set(k, String(v))
    })
    const qs = params.toString()
    return axios
      .post<ParsedFile>(`${endpoints.files.process}${qs ? `?${qs}` : ''}`, form, MULTIPART)
      .then((r) => r.data)
  },

  listJobs(params: ListJobsParams = {}): Promise<JobsPage> {
    return axios
      .get<JobsPage>(endpoints.files.jobs, { params })
      .then((r) => r.data)
  },

  getJob(id: string): Promise<Job> {
    return axios.get<Job>(endpoints.files.jobDetail(id)).then((r) => r.data)
  },

  /** POST /api/v1/jobs — upload files and start a batch over them. */
  createBatch(files: File[]): Promise<{ job_id: string }> {
    const form = new FormData()
    files.forEach((f) => form.append('file', f))
    return axios
      .post<{ job_id: string }>(endpoints.files.jobs, form, MULTIPART)
      .then((r) => r.data)
  },
}
