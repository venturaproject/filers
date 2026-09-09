import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { ColumnRule, FieldType, ValidationSchema } from '@/services/files-api'

const TYPES: (FieldType | '')[] = ['', 'string', 'integer', 'float', 'number', 'boolean']

export function SchemaBuilder({
  columns,
  value,
  onChange,
}: {
  columns: string[]
  value: ValidationSchema
  onChange: (schema: ValidationSchema) => void
}) {
  const setRule = (col: string, patch: Partial<ColumnRule> | null) => {
    const cols = { ...value.columns }
    if (patch === null) delete cols[col]
    else cols[col] = { ...cols[col], ...patch }
    onChange({ ...value, columns: cols })
  }

  return (
    <div className="space-y-2">
      {columns.map((col) => {
        const rule = value.columns[col]
        const on = rule !== undefined
        return (
          <div
            key={col}
            className="rounded-md border p-3"
            data-active={on}
          >
            <div className="flex items-center gap-2">
              <Checkbox
                checked={on}
                onCheckedChange={(c) => setRule(col, c ? {} : null)}
              />
              <span className="font-medium">{col}</span>
            </div>
            {on && (
              <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-2 pl-6 text-sm">
                <label className="flex items-center gap-1.5">
                  <Checkbox
                    checked={!!rule?.required}
                    onCheckedChange={(c) => setRule(col, { required: !!c })}
                  />
                  requerido
                </label>
                <label className="flex items-center gap-1.5">
                  <Checkbox
                    checked={!!rule?.unique}
                    onCheckedChange={(c) => setRule(col, { unique: !!c })}
                  />
                  único
                </label>
                <Select
                  value={rule?.type ?? ''}
                  onValueChange={(v) => setRule(col, { type: (v || undefined) as FieldType | undefined })}
                >
                  <SelectTrigger className="h-8 w-[130px]">
                    <SelectValue placeholder="tipo" />
                  </SelectTrigger>
                  <SelectContent>
                    {TYPES.map((t) => (
                      <SelectItem key={t || 'any'} value={t}>{t || 'cualquiera'}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  className="h-8 w-[220px]"
                  placeholder="regex"
                  value={rule?.regex ?? ''}
                  onChange={(e) => setRule(col, { regex: e.target.value || undefined })}
                />
                <Input
                  className="h-8 w-[220px]"
                  placeholder="valores permitidos (a, b, c)"
                  value={rule?.enum?.join(', ') ?? ''}
                  onChange={(e) =>
                    setRule(col, {
                      enum: e.target.value
                        ? e.target.value.split(',').map((s) => s.trim())
                        : undefined,
                    })
                  }
                />
                <Input
                  className="h-8 w-[90px]"
                  type="number"
                  placeholder="min"
                  value={rule?.min ?? ''}
                  onChange={(e) =>
                    setRule(col, { min: e.target.value === '' ? undefined : Number(e.target.value) })
                  }
                />
                <Input
                  className="h-8 w-[90px]"
                  type="number"
                  placeholder="max"
                  value={rule?.max ?? ''}
                  onChange={(e) =>
                    setRule(col, { max: e.target.value === '' ? undefined : Number(e.target.value) })
                  }
                />
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}
