import { ListFilter, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Separator } from '@/components/ui/separator'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useI18n } from '@/i18n/context'

export interface FilterGroup {
  key: string
  label: string
  value: string | undefined
  allLabel?: string
  options: { value: string; label: string }[]
  onChange: (value: string | undefined) => void
}

interface ListFilterPopoverProps {
  groups: FilterGroup[]
  onClearAll: () => void
}

/**
 * Filter button + popover in the reference-app style — a "Filtros" trigger
 * with a dot when any filter is active, opening single-select groups of
 * checkboxes. Used by the OCR Jobs / Documents lists.
 */
export function ListFilterPopover({ groups, onClearAll }: ListFilterPopoverProps) {
  const { t } = useI18n()
  const hasActiveFilters = groups.some((g) => g.value)

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" className="h-9 gap-1">
          <ListFilter className="h-3.5 w-3.5" />
          <span className="sr-only sm:not-sr-only sm:whitespace-nowrap">{t('filter')}</span>
          {hasActiveFilters && <span className="ml-0.5 h-1.5 w-1.5 rounded-full bg-primary" />}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-56 p-0">
        <div className="p-3">
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t('filter_by')}
          </p>
        </div>

        {groups.map((group) => (
          <div key={group.key}>
            <Separator />
            <div className="p-3">
              <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {group.label}
              </p>
              <div className="space-y-1.5">
                <label className="flex cursor-pointer select-none items-center gap-2 text-sm">
                  <Checkbox
                    checked={!group.value}
                    onCheckedChange={(checked) => {
                      if (checked) group.onChange(undefined)
                    }}
                  />
                  {group.allLabel ?? 'Todos'}
                </label>
                {group.options.map((option) => (
                  <label
                    key={option.value}
                    className="flex cursor-pointer select-none items-center gap-2 text-sm"
                  >
                    <Checkbox
                      checked={group.value === option.value}
                      onCheckedChange={(checked) =>
                        group.onChange(checked ? option.value : undefined)
                      }
                    />
                    {option.label}
                  </label>
                ))}
              </div>
            </div>
          </div>
        ))}

        {hasActiveFilters && (
          <>
            <Separator />
            <div className="p-3">
              <Button
                variant="ghost"
                size="sm"
                className="w-full text-xs text-muted-foreground"
                onClick={onClearAll}
              >
                <X className="mr-1 h-3 w-3" />
                {t('clear_filters')}
              </Button>
            </div>
          </>
        )}
      </PopoverContent>
    </Popover>
  )
}
