import React from 'react'
import { CheckSquare, ChevronDown, MoreVertical, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useI18n } from '@/i18n/context'

interface BulkSelectionBarProps {
  /** Show/hide bar — pass `selectedIds.size > 0` */
  visible: boolean
  selectedCount: number
  total: number
  allPageSelected: boolean
  selectAllRecords: boolean
  onSelectAllRecords: () => void
  onSelectPageOnly: () => void
  onClearSelection: () => void
  /** Bulk action items rendered inside the actions dropdown */
  actions: React.ReactNode
  /** Pre-computed count label e.g. "3 users selected" */
  countLabel: string
  /** Label for the "select all N records" link */
  selectAllLabel?: string
}

export function BulkSelectionBar({
  visible,
  selectedCount,
  total,
  allPageSelected,
  selectAllRecords,
  onSelectAllRecords,
  onSelectPageOnly,
  onClearSelection,
  actions,
  countLabel,
  selectAllLabel,
}: BulkSelectionBarProps) {
  const { t } = useI18n()

  if (!visible) return null

  return (
    <div className="flex items-center gap-3 rounded-lg border border-primary/20 bg-primary/5 px-4 py-2.5 mb-2">
      <CheckSquare className="h-4 w-4 text-primary shrink-0" />
      <span className="text-sm font-medium">{countLabel}</span>

      {allPageSelected && !selectAllRecords && selectedCount < total && selectAllLabel && (
        <>
          <div className="h-4 w-px bg-border" />
          <button
            type="button"
            className="text-sm text-primary underline-offset-4 hover:underline"
            onClick={onSelectAllRecords}
          >
            {selectAllLabel}
          </button>
        </>
      )}

      {selectAllRecords && (
        <>
          <div className="h-4 w-px bg-border" />
          <button
            type="button"
            className="text-sm text-primary underline-offset-4 hover:underline"
            onClick={onSelectPageOnly}
          >
            {t('select_this_page_only')}
          </button>
        </>
      )}

      <div className="h-4 w-px bg-border mx-1" />

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="h-8 gap-1.5 text-sm font-medium">
            <MoreVertical className="h-3.5 w-3.5" />
            {t('actions')}
            <ChevronDown className="h-3.5 w-3.5" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-48">
          {actions}
        </DropdownMenuContent>
      </DropdownMenu>

      <Button
        variant="ghost"
        size="sm"
        className="h-8 px-2 text-xs text-muted-foreground ml-auto"
        onClick={onClearSelection}
      >
        <X className="mr-1 h-3 w-3" />
        {t('deselect_all')}
      </Button>
    </div>
  )
}
