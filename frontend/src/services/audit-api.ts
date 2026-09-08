import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'

export interface AuditEvent {
  id: string
  created_at: string
  action: string
  actor_type: string
  actor_id: string | null
  actor_label: string | null
  target: string | null
  ip: string | null
  meta: Record<string, unknown> | null
}

export interface AuditListResponse {
  data: AuditEvent[]
  total: number
  page: number
  per_page: number
}

export interface AuditListParams {
  page?: number
  per_page?: number
  action?: string
  actor?: string
}

export const auditApi = {
  async list(params: AuditListParams): Promise<AuditListResponse> {
    const { data } = await axios.get<AuditListResponse>(endpoints.audit, { params })
    return data
  },
}
