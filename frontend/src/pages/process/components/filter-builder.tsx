import { Plus, X } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { FilterOp, Predicate } from '@/services/files-api'

const OPS: { value: FilterOp; label: string; needsValue: boolean }[] = [
  { value: 'eq', label: '=', needsValue: true },
  { value: 'ne', label: '≠', needsValue: true },
  { value: 'gt', label: '>', needsValue: true },
  { value: 'gte', label: '≥', needsValue: true },
  { value: 'lt', label: '<', needsValue: true },
  { value: 'lte', label: '≤', needsValue: true },
  { value: 'in', label: 'en lista', needsValue: true },
  { value: 'not_in', label: 'no en lista', needsValue: true },
  { value: 'matches', label: 'coincide (regex)', needsValue: true },
  { value: 'not_null', label: 'no vacío', needsValue: false },
  { value: 'is_null', label: 'vacío', needsValue: false },
]

export interface FilterRow {
  column: string
  op: FilterOp
  value: string
}

/** Turn the editable rows into the API's `filter` map. */
export function rowsToFilter(rows: FilterRow[]): Record<string, Predicate> {
  const out: Record<string, Predicate> = {}
  for (const r of rows) {
    if (!r.column) continue
    const spec = OPS.find((o) => o.value === r.op)
    if (spec?.needsValue && r.value === '') continue
    const num = Number(r.value)
    const isNum = r.value !== '' && !Number.isNaN(num)
    const p: Predicate = out[r.column] ?? {}
    switch (r.op) {
      case 'gt': case 'gte': case 'lt': case 'lte':
        p[r.op] = isNum ? num : 0
        break
      case 'in': case 'not_in':
        p[r.op] = r.value.split(',').map((s) => s.trim())
        break
      case 'is_null': p.is_null = true; break
      case 'not_null': p.not_null = true; break
      case 'eq': case 'ne':
        p[r.op] = isNum ? num : r.value
        break
      case 'matches': p.matches = r.value; break
    }
    out[r.column] = p
  }
  return out
}

export function FilterBuilder({
  columns,
  rows,
  onChange,
}: {
  columns: string[]
  rows: FilterRow[]
  onChange: (rows: FilterRow[]) => void
}) {
  const update = (i: number, patch: Partial<FilterRow>) =>
    onChange(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)))

  return (
    <div className="space-y-2">
      {rows.map((row, i) => {
        const needsValue = OPS.find((o) => o.value === row.op)?.needsValue ?? true
        return (
          <div key={i} className="flex flex-wrap items-center gap-2">
            <Select value={row.column} onValueChange={(v) => update(i, { column: v })}>
              <SelectTrigger className="h-9 w-[180px]">
                <SelectValue placeholder="Columna" />
              </SelectTrigger>
              <SelectContent>
                {columns.map((c) => (
                  <SelectItem key={c} value={c}>{c}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select value={row.op} onValueChange={(v) => update(i, { op: v as FilterOp })}>
              <SelectTrigger className="h-9 w-[150px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {OPS.map((o) => (
                  <SelectItem key={o.value} value={o.value}>{o.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            {needsValue && (
              <Input
                className="h-9 w-[200px]"
                placeholder={row.op === 'in' || row.op === 'not_in' ? 'a, b, c' : 'valor'}
                value={row.value}
                onChange={(e) => update(i, { value: e.target.value })}
              />
            )}
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-9 w-9"
              onClick={() => onChange(rows.filter((_, j) => j !== i))}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        )
      })}
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="gap-1"
        onClick={() =>
          onChange([...rows, { column: columns[0] ?? '', op: 'eq', value: '' }])
        }
      >
        <Plus className="h-4 w-4" /> Añadir filtro
      </Button>
    </div>
  )
}
