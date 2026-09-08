import {
  ChevronsLeft,
  ChevronsRight,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { CardFooter } from '@/components/ui/card'
import { useI18n } from '@/i18n/context'
import { PAGINATION_OPTIONS } from '@/lib/constants'

interface DataTablePaginationProps {
  currentPage: number
  lastPage: number
  perPage: number
  total: number
  selectedCount: number
  onPageChange: (page: number) => void
  onPerPageChange: (value: string) => void
}

export function DataTablePagination({
  currentPage,
  lastPage,
  perPage,
  total,
  selectedCount,
  onPageChange,
  onPerPageChange,
}: DataTablePaginationProps) {
  const { t } = useI18n()

  const start = total === 0 ? 0 : (currentPage - 1) * perPage + 1
  const end = Math.min(currentPage * perPage, total)

  return (
    <CardFooter className="flex items-center justify-between gap-4 py-4">
      <span className="text-sm text-muted-foreground">
        {selectedCount > 0
          ? t('n_rows_selected', { selected: selectedCount, total })
          : t('showing_range', { start, end, total })}
      </span>
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground whitespace-nowrap">
            {t('rows_per_page')}
          </span>
          <Select value={String(perPage)} onValueChange={onPerPageChange}>
            <SelectTrigger className="h-8 w-[70px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PAGINATION_OPTIONS.map((n) => (
                <SelectItem key={n} value={String(n)}>
                  {n}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <span className="text-sm text-muted-foreground whitespace-nowrap">
          {t('page_of', { current: currentPage, total: lastPage })}
        </span>
        <div className="flex items-center gap-1">
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={() => onPageChange(1)}
            disabled={currentPage === 1}
          >
            <ChevronsLeft className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={() => onPageChange(currentPage - 1)}
            disabled={currentPage === 1}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={() => onPageChange(currentPage + 1)}
            disabled={currentPage === lastPage}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={() => onPageChange(lastPage)}
            disabled={currentPage === lastPage}
          >
            <ChevronsRight className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </CardFooter>
  )
}
