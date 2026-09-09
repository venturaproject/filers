import { useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import { GitCompareArrows, Loader2, MinusCircle, Pencil, PlusCircle, Play, Equal } from 'lucide-react'
import { toast } from 'sonner'

import { Main } from '@/components/layout'
import { AuthenticatedLayout } from '@/layouts'
import { MetricStatCard } from '@/components/metric-stat-card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { FileDropzone } from '@/components/file-dropzone'
import { filesApi, type DiffReport } from '@/services/files-api'

function apiError(e: unknown, fallback: string): string {
  return (e as { response?: { data?: { error?: string } } })?.response?.data?.error ?? fallback
}

const cell = (v: unknown) => (v == null || v === '' ? '∅' : String(v))

function KeyedRows({
  rows,
}: {
  rows: { key: Record<string, unknown>; row: Record<string, unknown> }[]
}) {
  if (rows.length === 0) return <p className="p-4 text-sm text-muted-foreground">Nada</p>
  const cols = Object.keys(rows[0].row)
  return (
    <div className="max-h-96 overflow-auto">
      <Table>
        <TableHeader>
          <TableRow>
            {cols.map((c) => (
              <TableHead key={c}>{c}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((r, i) => (
            <TableRow key={i}>
              {cols.map((c) => (
                <TableCell key={c} className="text-sm">{cell(r.row[c])}</TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function ChangedRows({ report }: { report: DiffReport }) {
  if (report.changed.length === 0)
    return <p className="p-4 text-sm text-muted-foreground">Nada</p>
  return (
    <div className="max-h-96 overflow-auto">
      <Table>
        <TableHeader>
          <TableRow>
            {report.key.map((k) => (
              <TableHead key={k}>{k}</TableHead>
            ))}
            <TableHead>Cambios</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {report.changed.map((r, i) => (
            <TableRow key={i}>
              {report.key.map((k) => (
                <TableCell key={k} className="font-medium tabular-nums">{cell(r.key[k])}</TableCell>
              ))}
              <TableCell>
                <div className="space-y-1">
                  {Object.entries(r.changes).map(([col, ch]) => (
                    <div key={col} className="text-xs">
                      <span className="text-muted-foreground">{col}: </span>
                      <span className="rounded bg-red-100 px-1 line-through dark:bg-red-950/40">
                        {cell(ch.from)}
                      </span>
                      {' → '}
                      <span className="rounded bg-emerald-100 px-1 dark:bg-emerald-950/40">
                        {cell(ch.to)}
                      </span>
                    </div>
                  ))}
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

export default function ComparePage() {
  const [aFiles, setAFiles] = useState<File[]>([])
  const [bFiles, setBFiles] = useState<File[]>([])
  const a = aFiles[0] ?? null
  const b = bFiles[0] ?? null
  const [keys, setKeys] = useState<string[]>([])

  const headers = useQuery({
    queryKey: ['diff-headers', a?.name, b?.name, a?.size, b?.size],
    enabled: !!a && !!b,
    queryFn: async () => {
      const [ra, rb] = await Promise.all([
        filesApi.process(a as File, { max_rows: 1 }),
        filesApi.process(b as File, { max_rows: 1 }),
      ])
      return ra.columns.filter((c) => rb.columns.includes(c))
    },
  })
  const common = headers.data ?? []

  const diff = useMutation({
    mutationFn: () => filesApi.diff(a as File, b as File, keys),
    onError: (e) => toast.error(apiError(e, 'No se pudo comparar.')),
  })
  const report = diff.data?.report

  const toggleKey = (c: string) =>
    setKeys((prev) => (prev.includes(c) ? prev.filter((x) => x !== c) : [...prev, c]))

  return (
    <AuthenticatedLayout title="Comparar archivos">
      <Main>
        <div className="grid max-w-6xl flex-1 items-start gap-6">
          <div>
            <div className="flex items-center gap-2">
              <GitCompareArrows className="h-6 w-6 text-primary" />
              <h2 className="text-2xl font-bold tracking-tight">Comparar archivos</h2>
            </div>
            <p className="mt-1 text-muted-foreground">
              Sube dos versiones y Filers te dice qué filas se han añadido, borrado o cambiado,
              emparejándolas por una o varias columnas clave.
            </p>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Card>
              <CardHeader className="pb-3"><CardTitle className="text-sm">Archivo A (original)</CardTitle></CardHeader>
              <CardContent>
                <FileDropzone value={aFiles} onChange={setAFiles} multiple={false} maxSizeMb={100} />
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-3"><CardTitle className="text-sm">Archivo B (nuevo)</CardTitle></CardHeader>
              <CardContent>
                <FileDropzone value={bFiles} onChange={setBFiles} multiple={false} maxSizeMb={100} />
              </CardContent>
            </Card>
          </div>

          {a && b && (
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm">Columna(s) clave</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {headers.isLoading && (
                  <p className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" /> Leyendo cabeceras…
                  </p>
                )}
                <div className="flex flex-wrap gap-2">
                  {common.map((c) => (
                    <label
                      key={c}
                      className="flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1 text-sm data-[on=true]:border-primary data-[on=true]:bg-primary/10"
                      data-on={keys.includes(c)}
                    >
                      <Checkbox checked={keys.includes(c)} onCheckedChange={() => toggleKey(c)} />
                      {c}
                    </label>
                  ))}
                  {!headers.isLoading && common.length === 0 && (
                    <span className="text-sm text-muted-foreground">
                      Los archivos no comparten columnas.
                    </span>
                  )}
                </div>
                <Button
                  onClick={() => diff.mutate()}
                  disabled={keys.length === 0 || diff.isPending}
                  className="gap-2"
                >
                  {diff.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
                  Comparar
                </Button>
              </CardContent>
            </Card>
          )}

          {report && (
            <>
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                <MetricStatCard title="Añadidas" value={report.summary.added} subtitle="Solo en B" icon={PlusCircle} sparklineColor="#10b981" />
                <MetricStatCard title="Eliminadas" value={report.summary.removed} subtitle="Solo en A" icon={MinusCircle} sparklineColor="#f87171" />
                <MetricStatCard title="Cambiadas" value={report.summary.changed} subtitle="En ambos, distintas" icon={Pencil} sparklineColor="#f59e0b" />
                <MetricStatCard title="Sin cambios" value={report.summary.unchanged} subtitle="Idénticas" icon={Equal} sparklineColor="#6366f1" />
              </div>

              {(report.summary.columns_added.length > 0 || report.summary.columns_removed.length > 0) && (
                <p className="text-sm text-muted-foreground">
                  {report.summary.columns_added.length > 0 && (
                    <>Columnas nuevas: {report.summary.columns_added.map((c) => <Badge key={c} variant="outline" className="ml-1">{c}</Badge>)}. </>
                  )}
                  {report.summary.columns_removed.length > 0 && (
                    <>Columnas eliminadas: {report.summary.columns_removed.map((c) => <Badge key={c} variant="outline" className="ml-1">{c}</Badge>)}.</>
                  )}
                </p>
              )}
              {report.truncated && (
                <p className="text-xs text-muted-foreground">
                  Se muestran como máximo 5000 filas por categoría; los totales de arriba son exactos.
                </p>
              )}

              <Card>
                <CardContent className="p-0">
                  <Tabs defaultValue="changed">
                    <TabsList className="m-3">
                      <TabsTrigger value="changed">Cambiadas ({report.changed.length})</TabsTrigger>
                      <TabsTrigger value="added">Añadidas ({report.added.length})</TabsTrigger>
                      <TabsTrigger value="removed">Eliminadas ({report.removed.length})</TabsTrigger>
                    </TabsList>
                    <TabsContent value="changed"><ChangedRows report={report} /></TabsContent>
                    <TabsContent value="added"><KeyedRows rows={report.added} /></TabsContent>
                    <TabsContent value="removed"><KeyedRows rows={report.removed} /></TabsContent>
                  </Tabs>
                </CardContent>
              </Card>
            </>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
