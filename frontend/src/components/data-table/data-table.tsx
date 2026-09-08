import { flexRender, Table as TanstackTable } from '@tanstack/react-table'
import { GripVertical } from 'lucide-react'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'

interface DataTableProps<T> {
  table: TanstackTable<T>
  colCount: number
  emptyMessage?: string
  onDragStart: (id: string) => void
  onDrop: (id: string) => void
  fixedColumnIds?: string[]
  onRowClick?: (row: T) => void
  isRowActive?: (row: T) => boolean
}

export function DataTable<T>({
  table,
  colCount,
  emptyMessage = 'No results found.',
  onDragStart,
  onDrop,
  fixedColumnIds = ['select', 'actions'],
  onRowClick,
  isRowActive,
}: DataTableProps<T>) {
  return (
    <Table>
      <TableHeader>
        {table.getHeaderGroups().map((hg) => (
          <TableRow key={hg.id}>
            {hg.headers.map((header) => {
              const isFixed = fixedColumnIds.includes(header.column.id)
              return (
                <TableHead
                  key={header.id}
                  draggable={!isFixed}
                  onDragStart={!isFixed ? () => onDragStart(header.column.id) : undefined}
                  onDragOver={!isFixed ? (e) => e.preventDefault() : undefined}
                  onDrop={!isFixed ? () => onDrop(header.column.id) : undefined}
                  className={!isFixed ? 'cursor-grab select-none' : undefined}
                >
                  <div className="flex items-center gap-1">
                    {!isFixed && (
                      <GripVertical className="h-3.5 w-3.5 text-muted-foreground/50 flex-shrink-0" />
                    )}
                    {flexRender(header.column.columnDef.header, header.getContext())}
                  </div>
                </TableHead>
              )
            })}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {table.getRowModel().rows.length === 0 ? (
          <TableRow>
            <TableCell
              colSpan={colCount}
              className="h-24 text-center text-muted-foreground"
            >
              {emptyMessage}
            </TableCell>
          </TableRow>
        ) : (
          table.getRowModel().rows.map((row) => (
            <TableRow
              key={row.id}
              data-state={isRowActive?.(row.original) ? 'selected' : undefined}
              className={onRowClick ? 'cursor-pointer' : undefined}
              onClick={onRowClick ? () => onRowClick(row.original) : undefined}
            >
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))
        )}
      </TableBody>
    </Table>
  )
}
