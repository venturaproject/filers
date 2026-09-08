import { axios } from '@/lib/axios'
import { API_ENDPOINTS } from '@/config'
import { normalizeCollectionPayload, normalizeEntityPayload } from '@/lib/api-utils'

export type RoleMutationPayload = object

export const rolesApi = {
  async list(params?: Record<string, string | number>) {
    const { data } = await axios.get(API_ENDPOINTS.roles, { params })
    return data
  },

  async all(params?: Record<string, string | number>) {
    const { data } = await axios.get(API_ENDPOINTS.roles, { params: { per_page: 1000, ...params } })
    return normalizeCollectionPayload<Record<string, unknown>>(data)
  },

  async detail(id: string | number) {
    const { data } = await axios.get(`${API_ENDPOINTS.roles}/${id}`)
    return normalizeEntityPayload<Record<string, unknown>>(data)
  },

  async create(payload: RoleMutationPayload) {
    const { data } = await axios.post(API_ENDPOINTS.roles, payload)
    return data
  },

  async update(id: string | number, payload: RoleMutationPayload) {
    const { data } = await axios.put(`${API_ENDPOINTS.roles}/${id}`, payload)
    return data
  },

  async delete(id: string | number) {
    await axios.delete(`${API_ENDPOINTS.roles}/${id}`)
  },
}
