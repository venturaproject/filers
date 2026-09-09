import { useMemo, useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import {
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  Loader2,
  Play,
  Plus,
  Trash2,
  Workflow,
  XCircle,
} from 'lucide-react'
import { toast } from 'sonner'

import { Main } from '@/components/layout'
import { AuthenticatedLayout } from '@/layouts'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { FileDropzone } from '@/components/file-dropzone'
import {
  filesApi,
  type ConvertTarget,
  type PipelineStep,
  type ValidationSchema,
} from '@/services/files-api'

import { FilterBuilder, rowsToFilter, type FilterRow } from './components/filter-builder'
import { PreviewTable } from './components/preview-table'
import { SchemaBuilder } from './components/schema-builder'

type StepId = string
type Step =
  | { id: StepId; op: 'validate'; schema: ValidationSchema; fail_on_error: boolean }
  | { id: StepId; op: 'transform'; select: string[]; filters: FilterRow[]; limit: string }
  | { id: StepId; op: 'profile' }
  | { id: StepId; op: 'convert'; to: ConvertTarget }

const OP_LABEL: Record<Step['op'], string> = {
  validate: 'Validar',
  transform: 'Transformar',
  profile: 'Perfilar',
  convert: 'Convertir',
}

const uid = () => Math.random().toString(36).slice(2, 9)

function toApi(step: Step): PipelineStep {
  switch (step.op) {
    case 'validate':
      return { op: 'validate', schema: step.schema, fail_on_error: step.fail_on_error }
    case 'transform': {
      const spec: PipelineStep & { op: 'transform' } = { op: 'transform', spec: {} }
      if (step.select.length) spec.spec.select = step.select
      const f = rowsToFilter(step.filters)
      if (Object.keys(f).length) spec.spec.filter = f
      if (step.limit) spec.spec.limit = Number(step.limit)
      return spec
    }
    case 'profile':
      return { op: 'profile' }
    case 'convert':
      return { op: 'convert', to: step.to }
  }
}

function apiError(e: unknown, fallback: string): string {
  return (e as { response?: { data?: { error?: string } } })?.response?.data?.error ?? fallback
}

export default function PipelinePage() {
  const [files, setFiles] = useState<File[]>([])
  const file = files[0] ?? null
  const [steps, setSteps] = useState<Step[]>([])

  const headers = useQuery({
    queryKey: ['pipeline-headers', file?.name, file?.size],
    enabled: !!file,
    queryFn: () => filesApi.process(file as File, { max_rows: 1 }).then((r) => r.columns),
  })
  const columns = headers.data ?? []

  const patch = (id: StepId, p: Partial<Step>) =>
    setSteps((s) => s.map((st) => (st.id === id ? ({ ...st, ...p } as Step) : st)))
  const move = (i: number, dir: -1 | 1) =>
    setSteps((s) => {
      const j = i + dir
      if (j < 0 || j >= s.length) return s
      const copy = [...s]
      ;[copy[i], copy[j]] = [copy[j], copy[i]]
      return copy
    })
  const addStep = (op: Step['op']) => {
    const base = { id: uid() }
    setSteps((s) => [
      ...s,
      op === 'validate'
        ? { ...base, op, schema: { columns: {} }, fail_on_error: true }
        : op === 'transform'
          ? { ...base, op, select: [], filters: [], limit: '' }
          : op === 'convert'
            ? { ...base, op, to: 'csv' as ConvertTarget }
            : { ...base, op: 'profile' },
    ])
  }

  const pipeline = useMemo(() => ({ steps: steps.map(toApi) }), [steps])

  const run = useMutation({
    mutationFn: () => filesApi.pipeline(file as File, pipeline, { has_headers: true }),
    onSuccess: (res) => {
      if (res.kind === 'file') toast.success('Archivo descargado')
      if (res.kind === 'rejected') toast.error('La validación ha fallado')
    },
    onError: (e) => toast.error(apiError(e, 'No se pudo ejecutar el pipeline.')),
  })

  const convertNotLast = steps.some((s, i) => s.op === 'convert' && i !== steps.length - 1)

  return (
    <AuthenticatedLayout title="Pipeline">
      <Main>
        <div className="grid max-w-5xl flex-1 items-start gap-6">
          <div>
            <div className="flex items-center gap-2">
              <Workflow className="h-6 w-6 text-primary" />
              <h2 className="text-2xl font-bold tracking-tight">Pipeline</h2>
            </div>
            <p className="mt-1 text-muted-foreground">
              Encadena operaciones sobre un archivo en una sola petición: validar, transformar,
              perfilar y, al final, convertir.
            </p>
          </div>

          <Card>
            <CardContent className="pt-6">
              <FileDropzone value={files} onChange={setFiles} multiple={false} maxSizeMb={100} />
            </CardContent>
          </Card>

          {file && (
            <>
              <div className="space-y-3">
                {steps.map((step, i) => (
                  <Card key={step.id}>
                    <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0 pb-3">
                      <CardTitle className="flex items-center gap-2 text-sm">
                        <Badge variant="secondary">{i + 1}</Badge>
                        {OP_LABEL[step.op]}
                      </CardTitle>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => move(i, -1)}>
                          <ArrowUp className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => move(i, 1)}>
                          <ArrowDown className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 text-destructive"
                          onClick={() => setSteps((s) => s.filter((x) => x.id !== step.id))}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </CardHeader>
                    <CardContent>
                      {step.op === 'validate' && (
                        <div className="space-y-3">
                          <label className="flex items-center gap-1.5 text-sm">
                            <Checkbox
                              checked={step.fail_on_error}
                              onCheckedChange={(c) => patch(step.id, { fail_on_error: !!c })}
                            />
                            Detener el pipeline si el archivo no es válido
                          </label>
                          <SchemaBuilder
                            columns={columns}
                            value={step.schema}
                            onChange={(schema) => patch(step.id, { schema })}
                          />
                        </div>
                      )}
                      {step.op === 'transform' && (
                        <div className="space-y-3">
                          <div className="flex flex-wrap gap-2">
                            {columns.map((c) => (
                              <label
                                key={c}
                                className="flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1 text-sm data-[on=true]:border-primary data-[on=true]:bg-primary/10"
                                data-on={step.select.includes(c)}
                              >
                                <Checkbox
                                  checked={step.select.includes(c)}
                                  onCheckedChange={() =>
                                    patch(step.id, {
                                      select: step.select.includes(c)
                                        ? step.select.filter((x) => x !== c)
                                        : [...step.select, c],
                                    })
                                  }
                                />
                                {c}
                              </label>
                            ))}
                          </div>
                          <FilterBuilder
                            columns={columns}
                            rows={step.filters}
                            onChange={(filters) => patch(step.id, { filters })}
                          />
                          <Input
                            className="h-9 w-32"
                            type="number"
                            placeholder="límite"
                            value={step.limit}
                            onChange={(e) => patch(step.id, { limit: e.target.value })}
                          />
                        </div>
                      )}
                      {step.op === 'profile' && (
                        <p className="text-sm text-muted-foreground">
                          Añade el perfil de columnas al informe (no modifica los datos).
                        </p>
                      )}
                      {step.op === 'convert' && (
                        <Select
                          value={step.to}
                          onValueChange={(v) => patch(step.id, { to: v as ConvertTarget })}
                        >
                          <SelectTrigger className="h-9 w-[140px]">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {(['csv', 'json', 'ndjson', 'xlsx'] as ConvertTarget[]).map((t) => (
                              <SelectItem key={t} value={t}>{t.toUpperCase()}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      )}
                    </CardContent>
                  </Card>
                ))}
              </div>

              <div className="flex flex-wrap items-center gap-3">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="outline" className="gap-1">
                      <Plus className="h-4 w-4" /> Añadir paso
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent>
                    {(['validate', 'transform', 'profile', 'convert'] as Step['op'][]).map((op) => (
                      <DropdownMenuItem key={op} onClick={() => addStep(op)}>
                        {OP_LABEL[op]}
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
                <Button
                  className="gap-2"
                  disabled={steps.length === 0 || run.isPending || convertNotLast}
                  onClick={() => run.mutate()}
                >
                  {run.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
                  Ejecutar
                </Button>
                {convertNotLast && (
                  <span className="text-xs text-destructive">
                    «Convertir» debe ser el último paso.
                  </span>
                )}
              </div>

              {run.data?.kind === 'rejected' && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="flex items-center gap-2 text-base">
                      <XCircle className="h-5 w-5 text-destructive" /> Validación fallida
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <pre className="max-h-80 overflow-auto rounded bg-muted p-3 text-xs">
                      {JSON.stringify(run.data.steps, null, 2)}
                    </pre>
                  </CardContent>
                </Card>
              )}

              {run.data?.kind === 'file' && (
                <p className="flex items-center gap-2 text-sm text-emerald-600">
                  <CheckCircle2 className="h-4 w-4" /> Pipeline completado — archivo descargado.
                </p>
              )}

              {run.data?.kind === 'data' && (
                <>
                  <div className="flex flex-wrap gap-2 text-sm">
                    {run.data.body.steps.map((s, i) => {
                      const step = s as { op: string; ms: number }
                      return (
                        <Badge key={i} variant="outline" className="font-normal">
                          {step.op} · {step.ms} ms
                        </Badge>
                      )
                    })}
                  </div>
                  <PreviewTable
                    columns={run.data.body.columns}
                    data={run.data.body.data}
                    totalRows={run.data.body.stats.returned_rows}
                  />
                </>
              )}
            </>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
