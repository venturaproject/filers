import { useMemo, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import {
  AlertCircle,
  CheckCircle2,
  Download,
  FileSpreadsheet,
  Loader2,
  Play,
  Upload,
  XCircle,
} from 'lucide-react'
import { toast } from 'sonner'

import { Main } from '@/components/layout'
import { AuthenticatedLayout } from '@/layouts'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
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
import {
  filesApi,
  type ConvertTarget,
  type Timings,
  type TransformSpec,
  type ValidationSchema,
} from '@/services/files-api'

import { FilterBuilder, rowsToFilter, type FilterRow } from './components/filter-builder'
import { PreviewTable } from './components/preview-table'
import { ProfilePanel } from './components/profile-panel'
import { SchemaBuilder } from './components/schema-builder'

const PREVIEW_ROWS = 100
const ms = (n: number) => `${n.toLocaleString('es-ES')} ms`

function apiError(e: unknown, fallback: string): string {
  return (
    (e as { response?: { data?: { error?: string } } })?.response?.data?.error ?? fallback
  )
}

function TimingsCard({ timings: t, label }: { timings: Timings; label?: string }) {
  const starved = t.parse_ms > t.parse_cpu_ms * 1.5 && t.parse_ms - t.parse_cpu_ms > 200
  const rows: [string, string, string?][] = [
    ['Abrir archivo', ms(t.open_ms)],
    ['Leer hoja / stream', ms(t.read_ms), 'descompresión + parseo'],
    [label ?? 'Operación', ms(t.convert_ms)],
    ['Subida (red)', ms(t.upload_ms)],
  ]
  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base">Rendimiento</CardTitle>
        <CardDescription>
          <strong>{ms(t.parse_ms)}</strong> de reloj · <strong>{ms(t.parse_cpu_ms)}</strong> de CPU ·
          total <strong>{ms(t.total_ms)}</strong>
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-x-6 gap-y-1.5 text-sm sm:grid-cols-2">
          {rows.map(([l, v, hint]) => (
            <div key={l} className="flex items-baseline justify-between gap-2">
              <span className="text-muted-foreground">
                {l}
                {hint && <span className="ml-1 text-xs opacity-70">· {hint}</span>}
              </span>
              <span className="font-mono tabular-nums">{v}</span>
            </div>
          ))}
        </div>
        {starved && (
          <p className="rounded-md bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:bg-amber-950/30 dark:text-amber-400">
            El tiempo de reloj supera con mucho el de CPU: la máquina estaba saturada. El trabajo
            real son ~{ms(t.parse_cpu_ms)}.
          </p>
        )}
      </CardContent>
    </Card>
  )
}

// ── tab panels ──────────────────────────────────────────────────────────────

function ProfileTab({ file }: { file: File }) {
  const run = useMutation({
    mutationFn: () => filesApi.profile(file, { has_headers: true }),
    onError: (e) => toast.error(apiError(e, 'No se pudo analizar el archivo.')),
  })
  return (
    <div className="space-y-4">
      <Button onClick={() => run.mutate()} disabled={run.isPending} className="gap-2">
        {run.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
        Analizar columnas
      </Button>
      {run.data && (
        <>
          <TimingsCard timings={run.data.timings} label="Perfilado" />
          <ProfilePanel report={run.data.report} />
        </>
      )}
    </div>
  )
}

function ValidateTab({ file, columns }: { file: File; columns: string[] }) {
  const [schema, setSchema] = useState<ValidationSchema>({ columns: {} })
  const run = useMutation({
    mutationFn: () => filesApi.validate(file, schema, { has_headers: true }),
    onError: (e) => toast.error(apiError(e, 'No se pudo validar el archivo.')),
  })
  const report = run.data?.report
  const configured = Object.keys(schema.columns).length

  return (
    <div className="space-y-4">
      <SchemaBuilder columns={columns} value={schema} onChange={setSchema} />
      <Button
        onClick={() => run.mutate()}
        disabled={run.isPending || configured === 0}
        className="gap-2"
      >
        {run.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
        Validar {configured > 0 && `(${configured} columnas)`}
      </Button>

      {report && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2 text-base">
              {report.valid ? (
                <><CheckCircle2 className="h-5 w-5 text-emerald-500" /> Válido</>
              ) : (
                <><XCircle className="h-5 w-5 text-destructive" /> {report.error_count} errores</>
              )}
            </CardTitle>
            <CardDescription>
              {report.total_rows.toLocaleString('es-ES')} filas comprobadas
              {report.missing_columns.length > 0 &&
                ` · faltan: ${report.missing_columns.join(', ')}`}
              {report.errors_truncated && ' · lista de errores recortada'}
            </CardDescription>
          </CardHeader>
          {report.errors.length > 0 && (
            <CardContent className="p-0">
              <div className="max-h-96 overflow-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-16">Fila</TableHead>
                      <TableHead>Columna</TableHead>
                      <TableHead>Regla</TableHead>
                      <TableHead>Valor</TableHead>
                      <TableHead>Mensaje</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {report.errors.map((err, i) => (
                      <TableRow key={i}>
                        <TableCell className="tabular-nums">{err.row}</TableCell>
                        <TableCell className="font-medium">{err.column}</TableCell>
                        <TableCell><Badge variant="outline">{err.rule}</Badge></TableCell>
                        <TableCell className="max-w-[200px] truncate">{err.value || '∅'}</TableCell>
                        <TableCell className="text-muted-foreground">{err.message}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          )}
        </Card>
      )}
    </div>
  )
}

function TransformTab({ file, columns }: { file: File; columns: string[] }) {
  const [selected, setSelected] = useState<string[]>([])
  const [filters, setFilters] = useState<FilterRow[]>([])
  const [limit, setLimit] = useState('')

  const spec = useMemo<TransformSpec>(() => {
    const s: TransformSpec = {}
    if (selected.length && selected.length !== columns.length) s.select = selected
    const f = rowsToFilter(filters)
    if (Object.keys(f).length) s.filter = f
    if (limit) s.limit = Number(limit)
    return s
  }, [selected, filters, limit, columns.length])

  const preview = useMutation({
    mutationFn: () => filesApi.transform(file, spec, { has_headers: true }),
    onError: (e) => toast.error(apiError(e, 'No se pudo transformar el archivo.')),
  })
  const download = useMutation({
    mutationFn: (to: ConvertTarget) => filesApi.transform(file, spec, { has_headers: true, to }),
    onError: (e) => toast.error(apiError(e, 'No se pudo generar el archivo.')),
  })
  const result = preview.data && 'columns' in preview.data ? preview.data : null

  const toggle = (c: string) =>
    setSelected((prev) => (prev.includes(c) ? prev.filter((x) => x !== c) : [...prev, c]))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Columnas</CardTitle>
          <CardDescription>Sin selección = todas.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {columns.map((c) => (
            <label
              key={c}
              className="flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1 text-sm data-[on=true]:border-primary data-[on=true]:bg-primary/10"
              data-on={selected.includes(c)}
            >
              <Checkbox checked={selected.includes(c)} onCheckedChange={() => toggle(c)} />
              {c}
            </label>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Filtrar filas</CardTitle>
          <CardDescription>Se conservan las que cumplen todos los filtros.</CardDescription>
        </CardHeader>
        <CardContent>
          <FilterBuilder columns={columns} rows={filters} onChange={setFilters} />
        </CardContent>
      </Card>

      <div className="flex flex-wrap items-end gap-3">
        <div className="grid gap-1.5">
          <Label htmlFor="limit" className="text-xs">Límite de filas</Label>
          <Input
            id="limit"
            className="h-9 w-32"
            type="number"
            placeholder="todas"
            value={limit}
            onChange={(e) => setLimit(e.target.value)}
          />
        </div>
        <Button onClick={() => preview.mutate()} disabled={preview.isPending} className="gap-2">
          {preview.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
          Ver resultado
        </Button>
        {(['csv', 'json', 'ndjson', 'xlsx'] as ConvertTarget[]).map((t) => (
          <Button
            key={t}
            variant="outline"
            className="gap-1"
            disabled={download.isPending}
            onClick={() => download.mutate(t)}
          >
            <Download className="h-4 w-4" /> {t.toUpperCase()}
          </Button>
        ))}
      </div>

      {result && (
        <>
          <p className="text-sm text-muted-foreground">
            {result.matched_rows.toLocaleString('es-ES')} filas tras el filtro ·{' '}
            {result.stats.returned_rows.toLocaleString('es-ES')} devueltas ·{' '}
            {result.stats.columns} columnas
          </p>
          <PreviewTable columns={result.columns} data={result.data} totalRows={result.matched_rows} />
        </>
      )}
    </div>
  )
}

function ConvertTab({ file }: { file: File }) {
  const download = useMutation({
    mutationFn: (to: ConvertTarget) => filesApi.convert(file, to, { has_headers: true }),
    onSuccess: () => toast.success('Archivo descargado'),
    onError: (e) => toast.error(apiError(e, 'No se pudo convertir el archivo.')),
  })
  return (
    <div className="flex flex-wrap gap-3">
      {(['csv', 'json', 'ndjson', 'xlsx'] as ConvertTarget[]).map((t) => (
        <Button
          key={t}
          variant="outline"
          className="gap-2"
          disabled={download.isPending}
          onClick={() => download.mutate(t)}
        >
          <Download className="h-4 w-4" /> Descargar {t.toUpperCase()}
        </Button>
      ))}
    </div>
  )
}

// ── page ────────────────────────────────────────────────────────────────────

export default function ProcessPage() {
  const [files, setFiles] = useState<File[]>([])
  const file = files[0] ?? null

  const parse = useMutation({
    mutationFn: () => filesApi.process(file as File, { has_headers: true, max_rows: PREVIEW_ROWS }),
    onError: (e) => toast.error(apiError(e, 'No se pudo procesar el archivo.')),
  })
  const result = parse.data
  const columns = result?.columns ?? []

  return (
    <AuthenticatedLayout title="Banco de trabajo">
      <Main>
        <div className="grid max-w-6xl flex-1 items-start gap-6">
          <div>
            <div className="flex items-center gap-2">
              <FileSpreadsheet className="h-6 w-6 text-primary" />
              <h2 className="text-2xl font-bold tracking-tight">Banco de trabajo</h2>
            </div>
            <p className="mt-1 text-muted-foreground">
              Sube un Excel o CSV y aplícale operaciones: perfilado, validación, transformación y
              conversión. Para varios archivos usa{' '}
              <a href="/admin/jobs" className="underline underline-offset-2">Procesamientos</a>.
            </p>
          </div>

          <Card>
            <CardContent className="space-y-4 pt-6">
              <FileDropzone
                value={files}
                onChange={(f) => {
                  setFiles(f)
                  parse.reset()
                }}
                multiple={false}
                maxSizeMb={100}
                disabled={parse.isPending}
              />
              <Button
                onClick={() => file && parse.mutate()}
                disabled={!file || parse.isPending}
                className="gap-2"
              >
                {parse.isPending ? (
                  <><Loader2 className="h-4 w-4 animate-spin" /> Procesando…</>
                ) : (
                  <><Upload className="h-4 w-4" /> Cargar archivo</>
                )}
              </Button>
            </CardContent>
          </Card>

          {parse.isError && (
            <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/30 dark:text-red-400">
              <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
              <span>{apiError(parse.error, 'No se pudo procesar el archivo.')}</span>
            </div>
          )}

          {result && file && (
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">{file.name}</CardTitle>
                  <CardDescription className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">{result.format.toUpperCase()}</Badge>
                    <span>{result.stats.total_rows.toLocaleString('es-ES')} filas</span>
                    <span>·</span>
                    <span>{result.stats.columns} columnas</span>
                    <span>·</span>
                    <span>{ms(result.timings.total_ms)}</span>
                  </CardDescription>
                </CardHeader>
              </Card>

              <Tabs defaultValue="preview">
                <TabsList>
                  <TabsTrigger value="preview">Vista previa</TabsTrigger>
                  <TabsTrigger value="profile">Perfil</TabsTrigger>
                  <TabsTrigger value="validate">Validación</TabsTrigger>
                  <TabsTrigger value="transform">Transformar</TabsTrigger>
                  <TabsTrigger value="convert">Convertir</TabsTrigger>
                </TabsList>

                <TabsContent value="preview" className="space-y-4">
                  <TimingsCard timings={result.timings} label="Convertir celdas" />
                  <PreviewTable
                    columns={result.columns}
                    data={result.data}
                    totalRows={result.stats.total_rows}
                  />
                </TabsContent>
                <TabsContent value="profile">
                  <ProfileTab file={file} />
                </TabsContent>
                <TabsContent value="validate">
                  <ValidateTab file={file} columns={columns} />
                </TabsContent>
                <TabsContent value="transform">
                  <TransformTab file={file} columns={columns} />
                </TabsContent>
                <TabsContent value="convert">
                  <ConvertTab file={file} />
                </TabsContent>
              </Tabs>
            </>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
