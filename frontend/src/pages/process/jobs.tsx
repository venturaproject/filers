import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { CheckCircle2, Clock, Layers, Loader2, XCircle } from 'lucide-react'
import { MetricStatCard } from '@/components/metric-stat-card'
import { filesApi, type Job, type JobStatus, type ParsedSheet } from '@/services/files-api'

const STATUS_VARIANT: Record<JobStatus, 'default' | 'secondary' | 'outline' | 'destructive'> = {
  pending: 'outline',
  processing: 'secondary',
  done: 'default',
  error: 'destructive',
}

function SheetPreview({ sheet }: { sheet: ParsedSheet }) {
  const headers = sheet.rows.length > 0 ? Object.keys(sheet.rows[0]) : []
  const preview = sheet.rows.slice(0, 50)
  return (
    <div className="overflow-x-auto max-h-64 rounded border">
      <Table>
        <TableHeader>
          <TableRow>
            {headers.map((h) => <TableHead key={h} className="whitespace-nowrap text-xs">{h}</TableHead>)}
          </TableRow>
        </TableHeader>
        <TableBody>
          {preview.map((row, i) => (
            <TableRow key={i}>
              {headers.map((h) => (
                <TableCell key={h} className="whitespace-nowrap text-xs">
                  {row[h] == null ? <span className="text-muted-foreground">—</span> : String(row[h])}
                </TableCell>
              ))}
            </TableRow>
          ))}
          {sheet.row_count > 50 && (
            <TableRow>
              <TableCell colSpan={headers.length} className="text-center text-muted-foreground text-xs py-1">
                +{sheet.row_count - 50} filas más
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  )
}

export default function JobsPage() {
  const [selected, setSelected] = useState<string | null>(null)

  const detailQuery = useQuery({
    queryKey: ['file-job', selected],
    queryFn: () => filesApi.getJob(selected as string),
    enabled: !!selected,
    refetchInterval: (query) => {
      const status = query.state.data?.status
      return status === 'pending' || status === 'processing' ? 2000 : false
    },
  })

  const job = detailQuery.data

  return (
    <AuthenticatedLayout title="Trabajos batch">
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <div>
            <h2 className="text-2xl font-bold tracking-tight">Trabajos batch</h2>
            <p className="text-muted-foreground">
              Consulta el estado de trabajos de procesamiento en lote. Introduce el ID del trabajo para ver su resultado.
            </p>
          </div>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Consultar trabajo por ID</CardTitle>
            </CardHeader>
            <CardContent>
              <form
                className="flex gap-2"
                onSubmit={(e) => {
                  e.preventDefault()
                  const id = (e.currentTarget.elements.namedItem('job_id') as HTMLInputElement).value.trim()
                  if (id) setSelected(id)
                }}
              >
                <input
                  name="job_id"
                  placeholder="UUID del trabajo"
                  className="flex h-9 w-full max-w-xs rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  defaultValue={selected ?? ''}
                />
                <Button type="submit" size="sm">Ver estado</Button>
              </form>
            </CardContent>
          </Card>

          {selected && (
            <Card>
              <CardHeader>
                <CardTitle className="text-sm flex items-center gap-2">
                  Trabajo {selected.slice(0, 8)}…
                  {detailQuery.isLoading && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
                  {job && <Badge variant={STATUS_VARIANT[job.status]}>{job.status}</Badge>}
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                {detailQuery.isError && (
                  <p className="text-sm text-red-600">No se encontró el trabajo o hubo un error.</p>
                )}
                {job && (
                  <>
                    <div className="grid gap-1 text-sm">
                      {job.filename && <p><span className="text-muted-foreground">Archivo:</span> {job.filename}</p>}
                      <p><span className="text-muted-foreground">Creado:</span> {new Date(job.created_at).toLocaleString()}</p>
                      {job.finished_at && (
                        <p><span className="text-muted-foreground">Finalizado:</span> {new Date(job.finished_at).toLocaleString()}</p>
                      )}
                    </div>

                    {job.status === 'error' && job.error && (
                      <p className="text-sm text-red-600">{job.error}</p>
                    )}

                    {job.result && (
                      <>
                        <div className="flex flex-wrap gap-2 text-sm">
                          <Badge variant="outline">{job.result.format.toUpperCase()}</Badge>
                          <span>{job.result.stats.row_count} filas</span>
                          <span>·</span>
                          <span>{job.result.stats.col_count} columnas</span>
                          {job.result.stats.sheet_count > 1 && (
                            <><span>·</span><span>{job.result.stats.sheet_count} hojas</span></>
                          )}
                          <span>·</span>
                          <span>{job.result.stats.processing_ms} ms</span>
                        </div>
                        {job.result.sheets.map((sheet) => (
                          <div key={sheet.name}>
                            <p className="text-xs font-medium text-muted-foreground mb-1">{sheet.name}</p>
                            <SheetPreview sheet={sheet} />
                          </div>
                        ))}
                      </>
                    )}

                    {(job.status === 'pending' || job.status === 'processing') && (
                      <p className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Loader2 className="h-4 w-4 animate-spin" /> Procesando…
                      </p>
                    )}
                  </>
                )}
              </CardContent>
            </Card>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
