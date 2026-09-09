import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { ColumnProfile, ProfileReport } from '@/services/files-api'

const TYPE_COLOR: Record<string, string> = {
  integer: 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300',
  float: 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300',
  boolean: 'bg-violet-100 text-violet-700 dark:bg-violet-950 dark:text-violet-300',
  string: 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300',
  empty: 'bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300',
}

function ColumnCard({ c, totalRows }: { c: ColumnProfile; totalRows: number }) {
  const nullPct = totalRows ? Math.round((c.nulls / totalRows) * 100) : 0
  const topMax = c.top[0]?.count ?? 1
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center justify-between gap-2 text-sm">
          <span className="truncate" title={c.column}>{c.column || `Col ${c.index + 1}`}</span>
          <span className={`rounded px-1.5 py-0.5 text-xs font-medium ${TYPE_COLOR[c.inferred_type] ?? ''}`}>
            {c.inferred_type}
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2 text-xs">
        <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-muted-foreground">
          <span>Valores</span><span className="text-right tabular-nums text-foreground">{c.count.toLocaleString('es-ES')}</span>
          <span>Nulos</span><span className="text-right tabular-nums text-foreground">{c.nulls} ({nullPct}%)</span>
          <span>Distintos</span>
          <span className="text-right tabular-nums text-foreground">
            {c.distinct_capped ? '≥ ' : ''}{c.distinct.toLocaleString('es-ES')}
          </span>
          {c.min != null && (<><span>Mín</span><span className="text-right tabular-nums text-foreground">{c.min}</span></>)}
          {c.max != null && (<><span>Máx</span><span className="text-right tabular-nums text-foreground">{c.max}</span></>)}
          {c.mean != null && (<><span>Media</span><span className="text-right tabular-nums text-foreground">{c.mean.toFixed(2)}</span></>)}
        </div>
        {c.top.length > 0 && (
          <div className="space-y-1 pt-1">
            {c.top.slice(0, 5).map((t) => (
              <div key={t.value} className="flex items-center gap-2">
                <span className="w-28 truncate" title={t.value}>{t.value || '∅'}</span>
                <div className="h-1.5 flex-1 rounded bg-muted">
                  <div className="h-full rounded bg-primary/60" style={{ width: `${(t.count / topMax) * 100}%` }} />
                </div>
                <span className="w-10 text-right tabular-nums text-muted-foreground">{t.count}</span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

export function ProfilePanel({ report }: { report: ProfileReport }) {
  return (
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        {report.profile.length} columnas · {report.total_rows.toLocaleString('es-ES')} filas
      </p>
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {report.profile.map((c) => (
          <ColumnCard key={c.index} c={c} totalRows={report.total_rows} />
        ))}
      </div>
      {report.profile.length === 0 && (
        <Badge variant="outline">Sin columnas</Badge>
      )}
    </div>
  )
}
