import { isAxiosError } from 'axios'

/** Pull a `{ field: message | message[] }` bag out of an axios error response. */
export function extractApiErrors(error: unknown): Record<string, string | string[]> {
  if (isAxiosError(error) && error.response?.data && typeof error.response.data === 'object') {
    return error.response.data as Record<string, string | string[]>
  }
  return {}
}

/** First human-readable message from an axios error, or undefined. */
export function firstApiErrorMessage(error: unknown): string | undefined {
  const first = Object.values(extractApiErrors(error))[0]
  if (Array.isArray(first)) return typeof first[0] === 'string' ? first[0] : undefined
  return typeof first === 'string' ? first : undefined
}

export function normalizeEntityPayload<T>(payload: unknown): T | null {
  if (!payload || typeof payload !== 'object') return null

  const record = payload as Record<string, unknown>

  if (Array.isArray(record.results) && record.results.length === 1) {
    return record.results[0] as T
  }

  if (record.data && typeof record.data === 'object') {
    return record.data as T
  }

  return record as T
}

export function normalizeCollectionPayload<T>(payload: unknown): T[] {
  if (Array.isArray(payload)) return payload as T[]
  if (!payload || typeof payload !== 'object') return []

  const record = payload as Record<string, unknown>

  if (Array.isArray(record.results)) return record.results as T[]
  if (Array.isArray(record.data)) return record.data as T[]

  return []
}

export function normalizePaginatedPayload<T>(payload: unknown, fallbackPerPage = 20) {
  const record = (payload && typeof payload === 'object' ? payload : {}) as Record<string, unknown>
  const data = normalizeCollectionPayload<T>(payload)
  const total = Number(record.total ?? record.count ?? data.length ?? 0)
  const perPage = Number(record.per_page ?? fallbackPerPage)

  return {
    data,
    current_page: Number(record.current_page ?? record.page ?? 1),
    last_page: Number(record.last_page ?? Math.max(1, Math.ceil(total / Math.max(perPage, 1)))),
    per_page: perPage,
    total,
  }
}
