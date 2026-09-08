import { useState } from 'react'

export function useBulkSelection<TId extends string | number>(
  pageIds: TId[],
  total: number
) {
  const [selectedIds, setSelectedIds] = useState<Set<TId>>(new Set())
  const [selectAllRecords, setSelectAllRecords] = useState(false)

  const allPageSelected =
    pageIds.length > 0 && pageIds.every((id) => selectedIds.has(id))
  const selectedCount = selectAllRecords ? total : selectedIds.size

  const toggleSelectAll = (checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (checked) pageIds.forEach((id) => next.add(id))
      else pageIds.forEach((id) => next.delete(id))
      return next
    })
  }

  const toggleSelectRow = (id: TId, checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (checked) next.add(id)
      else next.delete(id)
      return next
    })
  }

  const clearSelection = () => {
    setSelectedIds(new Set())
    setSelectAllRecords(false)
  }

  return {
    selectedIds,
    selectAllRecords,
    setSelectAllRecords,
    selectedCount,
    allPageSelected,
    toggleSelectAll,
    toggleSelectRow,
    clearSelection,
  }
}
