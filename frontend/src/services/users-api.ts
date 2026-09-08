import { axios } from '@/lib/axios'
import { API_ENDPOINTS } from '@/config'
import { normalizeEntityPayload } from '@/lib/api-utils'

export type UserMutationPayload = object

export const usersApi = {
  async list(params?: Record<string, string>) {
    const { data } = await axios.get(API_ENDPOINTS.users, { params })
    return data
  },

  async detail(id: string | number) {
    const { data } = await axios.get(`${API_ENDPOINTS.users}/${id}`)
    return normalizeEntityPayload<Record<string, unknown>>(data)
  },

  async create(payload: UserMutationPayload) {
    const { data } = await axios.post(API_ENDPOINTS.users, payload)
    return data
  },

  async update(id: string | number, payload: UserMutationPayload) {
    const { data } = await axios.put(`${API_ENDPOINTS.users}/${id}`, payload)
    return data
  },

  async delete(id: string | number) {
    await axios.delete(`${API_ENDPOINTS.users}/${id}`)
  },
}
