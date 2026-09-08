import { axios } from '@/lib/axios'
import { API_ENDPOINTS } from '@/config'
import { normalizeCollectionPayload, normalizeEntityPayload } from '@/lib/api-utils'

export type PermissionMutationPayload = object

export const permissionsApi = {
  async list(params?: Record<string, string | number>) {
    const { data } = await axios.get(API_ENDPOINTS.permissions, { params })
    return data
  },

  async all(params?: Record<string, string | number>) {
    const { data } = await axios.get(API_ENDPOINTS.permissions, {
      params: { per_page: 1000, ...params },
    })
    return normalizeCollectionPayload<Record<string, unknown>>(data)
  },

  async detail(id: string | number) {
    const { data } = await axios.get(`${API_ENDPOINTS.permissions}/${id}`)
    return normalizeEntityPayload<Record<string, unknown>>(data)
  },

  async create(payload: PermissionMutationPayload) {
    const { data } = await axios.post(API_ENDPOINTS.permissions, payload)
    return data
  },

  async update(id: string | number, payload: PermissionMutationPayload) {
    const { data } = await axios.put(`${API_ENDPOINTS.permissions}/${id}`, payload)
    return data
  },

  async delete(id: string | number) {
    await axios.delete(`${API_ENDPOINTS.permissions}/${id}`)
  },
}
