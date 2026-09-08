import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'

export type JobStatus = 'pending' | 'processing' | 'done' | 'error'

export interface ParsedRow {
  [key: string]: string | number | boolean | null
}

export interface ParsedSheet {
  name: string
  rows: ParsedRow[]
  row_count: number
  col_count: number
}

export interface ParseStats {
  row_count: number
  col_count: number
  sheet_count: number
  processing_ms: number
  format: string
  filename: string
}

export interface ParsedFile {
  filename: string
  format: string
  sheets: ParsedSheet[]
  stats: ParseStats
}

export interface Job {
  id: string
  status: JobStatus
  filename: string | null
  created_at: string
  started_at: string | null
  finished_at: string | null
  error: string | null
  result: ParsedFile | null
}

export interface ProcessOptions {
  header_row?: boolean
  sheet?: string | null
  max_rows?: number | null
}

export const filesApi = {
  process(file: File, options?: ProcessOptions): Promise<ParsedFile> {
    const form = new FormData()
    form.append('file', file)
    if (options) form.append('options', JSON.stringify(options))
    return axios.post<ParsedFile>(endpoints.files.process, form, {
      headers: { 'Content-Type': 'multipart/form-data' },
    }).then((r) => r.data)
  },

  getJob(id: string): Promise<Job> {
    return axios.get<Job>(endpoints.files.jobDetail(id)).then((r) => r.data)
  },
}
