import axios from 'axios'
import { env } from '@/config'
import { getAuthAdapter } from './auth-adapter'

const axiosInstance = axios.create({
  baseURL: env?.PUBLIC_API_URL ?? '',
  withCredentials: true,
  headers: { 'Content-Type': 'application/json' },
})

axiosInstance.interceptors.request.use((config) => {
  if (config.method && ['post', 'put', 'patch', 'delete'].includes(config.method)) {
    Object.assign(config.headers, getAuthAdapter().getRequestHeaders())
  }
  return config
})

// Queue requests that arrive while a token refresh is in progress, then replay them all.
type QueueEntry = { resolve: () => void; reject: (err: unknown) => void }
let isRefreshing = false
let waitingQueue: QueueEntry[] = []

const flushQueue = (error: unknown) => {
  waitingQueue.forEach((entry) => (error ? entry.reject(error) : entry.resolve()))
  waitingQueue = []
}

axiosInstance.interceptors.response.use(
  (response) => response,
  async (error) => {
    const original = error.config
    if (error.response?.status !== 401 || original._retry) {
      return Promise.reject(error)
    }

    if (isRefreshing) {
      // Park this request; it will be replayed once the ongoing refresh finishes.
      return new Promise<void>((resolve, reject) => {
        waitingQueue.push({ resolve, reject })
      }).then(() => axiosInstance(original))
    }

    original._retry = true
    isRefreshing = true

    try {
      const refreshed = await getAuthAdapter().refresh()
      isRefreshing = false

      if (refreshed) {
        flushQueue(null)
        return axiosInstance(original)
      }

      flushQueue(error)
    } catch (refreshError) {
      isRefreshing = false
      flushQueue(refreshError)
    }

    const { useAuthStore } = await import('./auth')
    useAuthStore.getState().logout()
    getAuthAdapter().onUnauthorized()
    return Promise.reject(error)
  },
)

export { axiosInstance as axios }
