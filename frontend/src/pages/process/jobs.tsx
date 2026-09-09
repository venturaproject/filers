import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useSearchParams } from 'react-router-dom'
import { getCoreRowModel, useReactTable } from '@tanstack/react-table'
import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { MetricStatCard } from '@/components/metric-stat-card'
import { DataTable, DataTablePagination, DataTableViewOptions } from '@/components/data-table'
import { ListFilterPopover } from '@/components/list-filter-popover'
import { FileDropzone } from '@/components/file-dropzone'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Sheet,
  SheetContent,
  SheetDescription,
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
import {
  CheckCircle2,
  Clock,
  Download,
  Layers,
  Loader2,
  Plus,
  RefreshCw,
  Upload,
  X,
  XCircle,
} from 'lucide-react'
import { useI18n } from '@/i18n/context'
import { useTableFilters } from '@/hooks/use-table-filters'
import { useColumnReorder } from '@/hooks/use-column-reorder'
import {
  filesApi,
  type ConvertTarget,
  type JobOrigin,
  type JobStatus,
  type JobSummary,
} from '@/services/files-api'
import {
  KIND_LABEL,
  OPERATION_LABEL,
  ORIGIN_LABEL,
  STATUS_LABEL,
  STATUS_VARIANT,
  buildJobsColumns,
  formatDuration,
  jobColumnLabels,
} from './jobs-columns'

interface JobFilters {
  search?: string
  status?: string
  origin?: string
  page?: string
  per_page?: string
  [key: string]: string | undefined
}

export default function JobsPage() {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const [searchParams] = useSearchParams()
  const urlFilters = Object.fromEntries(searchParams.entries()) as JobFilters

  const [selected, setSelected] = useState<string | null>(null)
  const [newOpen, setNewOpen] = useState(false)
  const [newFiles, setNewFiles] = useState<File[]>([])
  const [outputOn, setOutputOn] = useState(false)
  const [outputFormat, setOutputFormat] = useState<ConvertTarget>('csv')

  const {
    filters,
    searchTerm,
    navigate: navigateFilters,
    handleSearch,
    handlePageChange,
    handlePerPageChange,
    perPage,
  } = useTableFilters<JobFilters>({
    basePath: '/admin/jobs',
    initialFilters: urlFilters,
    initialPerPage: Number(urlFilters.per_page) || 20,
  })

  const page = Number(urlFilters.page) || 1
  const currentPerPage = Number(urlFilters.per_page) || perPage

  const jobsQuery = useQuery({
    queryKey: ['batch-jobs', urlFilters],
    queryFn: () =>
      filesApi.listJobs({
        page,
        per_page: currentPerPage,
        status: urlFilters.status,
        origin: urlFilters.origin,
        search: urlFilters.search,
      }),
    refetchInterval: (query) =>
      query.state.data?.data.some((j) => j.status === 'pending' || j.status === 'running')
        ? 2500
        : false,
  })

  const detailQuery = useQuery({
    queryKey: ['batch-job', selected],
    queryFn: () => filesApi.getJob(selected as string),
    enabled: !!selected,
    refetchInterval: (query) =>
      query.state.data && ['pending', 'running'].includes(query.state.data.status) ? 2000 : false,
  })

  const upload = useMutation({
    mutationFn: (files: File[]) =>
      filesApi.createBatch(files, outputOn ? { to: outputFormat } : undefined),
    onSuccess: (res) => {
      setNewOpen(false)
      setNewFiles([])
      setOutputOn(false)
      setSelected(res.job_id)
      queryClient.invalidateQueries({ queryKey: ['batch-jobs'] })
    },
  })

  const downloadResult = useMutation({
    mutationFn: (name: string) => filesApi.downloadJobResult(selected as string, name),
  })

  const list = jobsQuery.data
  const jobs: JobSummary[] = list?.data ?? []
  const stats = list?.stats
  const total = list?.total ?? 0
  const lastPage = list?.last_page ?? 1

  const columns = buildJobsColumns()
  const { columnOrder, columnVisibility, setColumnVisibility, handleDragStart, handleDrop } =
    useColumnReorder(columns.map((c) => c.id as string))

  const table = useReactTable({
    data: jobs,
    columns,
    state: { columnOrder, columnVisibility },
    onColumnOrderChange: () => {},
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
  })

  const job = detailQuery.data

  const resultsQuery = useQuery({
    queryKey: ['batch-job-results', selected],
    queryFn: () => filesApi.jobResults(selected as string),
    enabled: !!selected && job?.kind === 'batch',
    refetchInterval: () =>
      job && !['completed', 'failed'].includes(job.status) ? 3000 : false,
  })

  return (
    <AuthenticatedLayout title="Procesamientos">
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-2xl font-bold tracking-tight">Procesamientos</h2>
              <p className="text-muted-foreground">
                Trazabilidad de todo el procesamiento de archivos — subidas del panel, llamadas a
                la API y trabajos en lote. Haz clic en una fila para ver el detalle por archivo.
              </p>
            </div>
            <Button className="gap-2" onClick={() => setNewOpen(true)}>
              <Plus className="h-4 w-4" /> Nuevo procesamiento
            </Button>
          </div>

          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <MetricStatCard
              title="En cola"
              value={stats?.pending ?? 0}
              subtitle="Pendientes de procesar"
              icon={Clock}
              sparklineColor="#f59e0b"
            />
            <MetricStatCard
              title="Procesando"
              value={stats?.running ?? 0}
              subtitle="Ahora mismo"
              icon={Layers}
              sparklineColor="#6366f1"
            />
            <MetricStatCard
              title="Completados"
              value={stats?.completed ?? 0}
              subtitle={
                stats?.avg_ms ? `duración media ${formatDuration(stats.avg_ms)}` : 'Sin datos'
              }
              icon={CheckCircle2}
              sparklineColor="#10b981"
            />
            <MetricStatCard
              title="Fallidos"
              value={stats?.failed ?? 0}
              subtitle="Con errores"
              icon={XCircle}
              sparklineColor="#f87171"
            />
          </div>

          <Card>
            <CardHeader>
              <div className="flex flex-col gap-3">
                <div className="flex items-center gap-2">
                  <Layers className="h-5 w-5" />
                  <CardTitle>Historial</CardTitle>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <div className="relative">
                    <Input
                      placeholder={t('filter_placeholder') || 'Buscar…'}
                      value={searchTerm}
                      onChange={(e) => handleSearch(e.target.value)}
                      className="w-64 pr-9"
                    />
                    {searchTerm && (
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 text-muted-foreground"
                        onClick={() => handleSearch('')}
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                  <ListFilterPopover
                    groups={[
                      {
                        key: 'status',
                        label: 'Estado',
                        allLabel: 'Todos los estados',
                        value: urlFilters.status,
                        options: (['pending', 'running', 'completed', 'failed'] as JobStatus[]).map(
                          (s) => ({ value: s, label: STATUS_LABEL[s] }),
                        ),
                        onChange: (v) =>
                          navigateFilters({ ...filters, status: v || undefined, page: '1' }),
                      },
                      {
                        key: 'origin',
                        label: 'Origen',
                        allLabel: 'Todos los orígenes',
                        value: urlFilters.origin,
                        options: (
                          ['admin', 'api_key', 'oauth_client', 'service'] as JobOrigin[]
                        ).map((o) => ({ value: o, label: ORIGIN_LABEL[o] })),
                        onChange: (v) =>
                          navigateFilters({ ...filters, origin: v || undefined, page: '1' }),
                      },
                    ]}
                    onClearAll={() =>
                      navigateFilters({
                        ...filters,
                        status: undefined,
                        origin: undefined,
                        page: '1',
                      })
                    }
                  />
                  <div className="ml-auto flex items-center gap-2">
                    <DataTableViewOptions table={table} columnLabels={jobColumnLabels} />
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-9 gap-2"
                      onClick={() => jobsQuery.refetch()}
                    >
                      <RefreshCw className="h-3.5 w-3.5" /> Actualizar
                    </Button>
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-x-auto">
                <DataTable
                  table={table}
                  colCount={columns.length}
                  emptyMessage={jobsQuery.isLoading ? 'Cargando…' : 'Sin trabajos todavía'}
                  onDragStart={handleDragStart}
                  onDrop={handleDrop}
                  fixedColumnIds={[]}
                  onRowClick={(j) => setSelected(j.id)}
                  isRowActive={(j) => j.id === selected}
                />
              </div>
            </CardContent>
            <DataTablePagination
              currentPage={page}
              lastPage={lastPage}
              perPage={currentPerPage}
              total={total}
              selectedCount={0}
              onPageChange={handlePageChange}
              onPerPageChange={handlePerPageChange}
            />
          </Card>
        </div>
      </Main>

      <Dialog
        open={newOpen}
        onOpenChange={(v) => {
          setNewOpen(v)
          if (!v) {
            setNewFiles([])
            upload.reset()
          }
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Nuevo procesamiento</DialogTitle>
            <DialogDescription>
              Sube uno o varios archivos Excel/CSV. Se procesan en segundo plano y aparecen en
              el historial.
            </DialogDescription>
          </DialogHeader>

          <FileDropzone
            value={newFiles}
            onChange={setNewFiles}
            multiple
            maxSizeMb={100}
            disabled={upload.isPending}
          />

          <div className="rounded-md border p-3">
            <label className="flex items-center gap-2 text-sm font-medium">
              <Checkbox
                checked={outputOn}
                onCheckedChange={(c) => setOutputOn(!!c)}
                disabled={upload.isPending}
              />
              Generar un archivo convertido por entrada
            </label>
            {outputOn && (
              <div className="mt-3 flex items-center gap-2 pl-6 text-sm">
                <span className="text-muted-foreground">Formato</span>
                <Select
                  value={outputFormat}
                  onValueChange={(v) => setOutputFormat(v as ConvertTarget)}
                >
                  <SelectTrigger className="h-8 w-[120px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {(['csv', 'json', 'ndjson', 'xlsx'] as ConvertTarget[]).map((t) => (
                      <SelectItem key={t} value={t}>{t.toUpperCase()}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <span className="text-xs text-muted-foreground">
                  descargable en el detalle del trabajo
                </span>
              </div>
            )}
          </div>

          {upload.isError && (
            <p className="text-sm text-destructive">
              {(upload.error as { response?: { data?: { error?: string } } })?.response?.data
                ?.error ?? 'No se pudo iniciar el procesamiento.'}
            </p>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={() => setNewOpen(false)} disabled={upload.isPending}>
              Cancelar
            </Button>
            <Button
              className="gap-2"
              disabled={newFiles.length === 0 || upload.isPending}
              onClick={() => upload.mutate(newFiles)}
            >
              {upload.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" /> Procesando…
                </>
              ) : (
                <>
                  <Upload className="h-4 w-4" /> Procesar {newFiles.length > 0 && `(${newFiles.length})`}
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Sheet open={!!selected} onOpenChange={(v) => !v && setSelected(null)}>
        <SheetContent side="right" className="w-full overflow-y-auto sm:max-w-2xl">
          {detailQuery.isLoading && (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" /> Cargando…
            </p>
          )}
          {detailQuery.isError && (
            <p className="text-sm text-destructive">No se encontró el trabajo.</p>
          )}
          {job && (
            <>
              <SheetHeader>
                <SheetTitle className="flex flex-wrap items-center gap-2">
                  {job.label ?? `Trabajo ${job.id.slice(0, 8)}`}
                  <Badge variant={STATUS_VARIANT[job.status]}>{STATUS_LABEL[job.status]}</Badge>
                  <Badge variant="outline" className="font-normal">
                    {OPERATION_LABEL[job.operation] ?? job.operation}
                  </Badge>
                </SheetTitle>
                <SheetDescription className="font-mono text-xs">{job.id}</SheetDescription>
              </SheetHeader>

              <div className="mt-6 space-y-5">
                <dl className="grid grid-cols-2 gap-x-4 gap-y-3 text-sm">
                  <div>
                    <dt className="text-muted-foreground">Modo</dt>
                    <dd>{KIND_LABEL[job.kind]}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Origen</dt>
                    <dd>
                      {ORIGIN_LABEL[job.origin]}
                      {job.actor && (
                        <span className="block text-xs text-muted-foreground">{job.actor}</span>
                      )}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Archivos</dt>
                    <dd>
                      {job.files_processed} ok
                      {job.files_failed > 0 && ` · ${job.files_failed} con error`} / {job.files_total}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Filas totales</dt>
                    <dd className="tabular-nums">{job.total_rows.toLocaleString('es-ES')}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Duración</dt>
                    <dd className="tabular-nums">{formatDuration(job.duration_ms)}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Creado</dt>
                    <dd>{new Date(job.created_at).toLocaleString('es-ES')}</dd>
                  </div>
                  {job.completed_at && (
                    <div>
                      <dt className="text-muted-foreground">Finalizado</dt>
                      <dd>{new Date(job.completed_at).toLocaleString('es-ES')}</dd>
                    </div>
                  )}
                </dl>

                {job.error && (
                  <p className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                    {job.error}
                  </p>
                )}

                {['pending', 'running'].includes(job.status) && (
                  <p className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" /> Procesando…
                  </p>
                )}

                {job.results.length > 0 && (
                  <div>
                    <p className="mb-2 text-sm font-medium">Resultado por archivo</p>
                    <div className="overflow-x-auto rounded-md border">
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>Archivo</TableHead>
                            <TableHead className="text-right">Filas</TableHead>
                            <TableHead className="text-right">Columnas</TableHead>
                            <TableHead className="text-right">Tiempo</TableHead>
                            <TableHead>Estado</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {job.results.map((r) => (
                            <TableRow key={r.file}>
                              <TableCell className="max-w-[220px] truncate font-medium">
                                {r.file}
                                {r.error && (
                                  <span className="block text-xs font-normal text-destructive">
                                    {r.error}
                                  </span>
                                )}
                              </TableCell>
                              <TableCell className="text-right tabular-nums">
                                {r.rows.toLocaleString('es-ES')}
                              </TableCell>
                              <TableCell className="text-right tabular-nums">{r.columns}</TableCell>
                              <TableCell className="text-right tabular-nums">
                                {formatDuration(r.elapsed_ms)}
                                {r.timings && (
                                  <span className="block text-xs font-normal text-muted-foreground">
                                    CPU {r.timings.parse_cpu_ms} ms · leer{' '}
                                    {r.timings.read_ms} ms
                                  </span>
                                )}
                              </TableCell>
                              <TableCell>
                                <Badge variant={r.status === 'ok' ? 'default' : 'destructive'}>
                                  {r.status === 'ok' ? 'OK' : 'Error'}
                                </Badge>
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </div>
                  </div>
                )}

                {(resultsQuery.data?.results.length ?? 0) > 0 && (
                  <div>
                    <p className="mb-2 text-sm font-medium">Archivos generados</p>
                    <ul className="divide-y rounded-md border">
                      {resultsQuery.data!.results.map((f) => (
                        <li
                          key={f.name}
                          className="flex items-center justify-between gap-3 px-3 py-2 text-sm"
                        >
                          <span className="truncate font-medium">{f.name}</span>
                          <span className="shrink-0 text-xs text-muted-foreground">
                            {(f.size_bytes / 1024).toFixed(1)} KB
                          </span>
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-7 shrink-0 gap-1"
                            disabled={downloadResult.isPending}
                            onClick={() => downloadResult.mutate(f.name)}
                          >
                            <Download className="h-3.5 w-3.5" /> Descargar
                          </Button>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            </>
          )}
        </SheetContent>
      </Sheet>
    </AuthenticatedLayout>
  )
}
