import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { SEARCH_DEBOUNCE_MS } from '@/lib/constants'

interface UseTableFiltersOptions<T> {
  basePath: string
  initialFilters: T
  initialPerPage: number
  onNavigate?: () => void
}

function buildUrl(base: string, params: Record<string, any>): string {
  const entries = Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '')
  if (!entries.length) return base
  return base + '?' + new URLSearchParams(entries.map(([k, v]) => [k, String(v)])).toString()
}

export function useTableFilters<T extends Record<string, string | undefined>>({
  basePath,
  initialFilters,
  initialPerPage,
  onNavigate,
}: UseTableFiltersOptions<T>) {
  const navigate = useNavigate()
  const [filters, setFilters] = useState<T>(initialFilters)
  const [searchTerm, setSearchTerm] = useState(initialFilters?.search ?? '')
  const [perPage, setPerPage] = useState(initialPerPage)
  const debounceTimer = useRef<ReturnType<typeof setTimeout>>(undefined)

  useEffect(() => () => clearTimeout(debounceTimer.current), [])

  const navigateWithFilters = useCallback((newFilters: T) => {
    setFilters(newFilters)
    onNavigate?.()
    navigate(buildUrl(basePath, newFilters as Record<string, any>), { replace: true })
  }, [basePath, onNavigate, navigate])

  const handleSearch = useCallback(
    (value: string) => {
      setSearchTerm(value)
      clearTimeout(debounceTimer.current)
      debounceTimer.current = setTimeout(() => {
        navigateWithFilters({ ...filters, search: value || undefined })
      }, SEARCH_DEBOUNCE_MS)
    },
    [filters, navigateWithFilters],
  )

  const handlePageChange = (page: number) => {
    navigate(buildUrl(basePath, { ...filters, page, per_page: perPage } as Record<string, any>), { replace: true })
  }

  const handlePerPageChange = (value: string) => {
    const newPerPage = parseInt(value)
    setPerPage(newPerPage)
    navigate(buildUrl(basePath, { ...filters, page: 1, per_page: newPerPage } as Record<string, any>), { replace: true })
  }

  return {
    filters,
    searchTerm,
    setSearchTerm,
    perPage,
    navigate: navigateWithFilters,
    handleSearch,
    handlePageChange,
    handlePerPageChange,
  }
}
