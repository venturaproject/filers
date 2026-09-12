import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Checkbox } from '@/components/ui/checkbox'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { AlertCircle, Loader2, ScanText, Sparkles } from 'lucide-react'
import { FileDropzone } from '@/components/file-dropzone'
import { ocrApi, type ExtractResult, type OcrResult } from '@/services/ocr-api'

type ApiErrorLike = { response?: { status?: number; data?: { error?: string } } }

/** The API's `{ error: "..." }` body, translated for the common cases. */
function friendlyError(error: unknown, fallback: string): string {
  const err = error as ApiErrorLike
  if (err?.response?.status === 503) {
    return 'La IA no está configurada en este servidor (falta OCR_LLM_API_KEY en el entorno).'
  }
  return err?.response?.data?.error ?? fallback
}

function ErrorBox({ message }: { message: string }) {
  return (
    <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/30 dark:text-red-400">
      <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
      <span>{message}</span>
    </div>
  )
}

function UsageBadges({ usage }: { usage: OcrResult['usage'] }) {
  if (!usage) return null
  return (
    <>
      <Badge variant="outline">{usage.prompt_tokens.toLocaleString('es-ES')} prompt</Badge>
      <Badge variant="outline">{usage.completion_tokens.toLocaleString('es-ES')} respuesta</Badge>
      <Badge variant="outline">{usage.total_tokens.toLocaleString('es-ES')} tokens totales</Badge>
    </>
  )
}

// ── OCR tab ───────────────────────────────────────────────────────────────────

function OcrTab() {
  const [files, setFiles] = useState<File[]>([])
  const [prompt, setPrompt] = useState('')
  const file = files[0] ?? null

  const run = useMutation({
    mutationFn: () => ocrApi.ocr(file as File, prompt),
  })

  return (
    <div className="grid gap-6">
      <Card>
        <CardContent className="space-y-4 pt-6">
          <FileDropzone
            value={files}
            onChange={(f) => {
              setFiles(f)
              run.reset()
            }}
            accept="image/png,image/jpeg,image/gif,image/webp"
            multiple={false}
            maxSizeMb={20}
            hint="PNG · JPEG · GIF · WEBP"
            disabled={run.isPending}
          />
          <div className="space-y-1">
            <Label htmlFor="ocr-prompt">Instrucción (opcional)</Label>
            <Textarea
              id="ocr-prompt"
              rows={2}
              placeholder="Por defecto: transcribir todo el texto visible, literal."
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              disabled={run.isPending}
            />
          </div>
          <Button
            onClick={() => file && run.mutate()}
            disabled={!file || run.isPending}
            className="gap-2"
          >
            {run.isPending ? (
              <><Loader2 className="h-4 w-4 animate-spin" /> Leyendo imagen…</>
            ) : (
              <><ScanText className="h-4 w-4" /> Extraer texto</>
            )}
          </Button>
        </CardContent>
      </Card>

      {run.isError && <ErrorBox message={friendlyError(run.error, 'No se pudo leer la imagen.')} />}

      {run.data && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Texto transcrito</CardTitle>
            <CardDescription className="flex flex-wrap items-center gap-2">
              <Badge variant="outline">{run.data.model}</Badge>
              <UsageBadges usage={run.data.usage} />
            </CardDescription>
          </CardHeader>
          <CardContent>
            <pre className="max-h-96 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-4 text-sm">
              {run.data.text || '(sin texto)'}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

// ── Extract tab ───────────────────────────────────────────────────────────────

const DEFAULT_FIELDS = 'name, quantity, unit_price, total'

function ItemsTable({ items }: { items: ExtractResult['items'] }) {
  if (items.length === 0) {
    return <p className="text-sm text-muted-foreground">No se encontró ningún elemento.</p>
  }
  const columns = Array.from(new Set(items.flatMap((it) => Object.keys(it))))
  return (
    <div className="max-h-96 overflow-auto rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            {columns.map((c) => (
              <TableHead key={c} className="whitespace-nowrap">{c}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {items.map((item, i) => (
            <TableRow key={i}>
              {columns.map((c) => (
                <TableCell key={c} className="whitespace-nowrap text-sm">
                  {item[c] == null ? (
                    <span className="text-muted-foreground">—</span>
                  ) : (
                    String(item[c])
                  )}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function ExtractTab() {
  const [files, setFiles] = useState<File[]>([])
  const [instruction, setInstruction] = useState('')
  const [fields, setFields] = useState(DEFAULT_FIELDS)
  const [redactPii, setRedactPii] = useState(false)
  const file = files[0] ?? null

  const run = useMutation({
    mutationFn: () =>
      ocrApi.extract(file as File, {
        instruction: instruction.trim() || undefined,
        fields: fields
          .split(',')
          .map((f) => f.trim())
          .filter(Boolean),
        redact_pii: redactPii,
      }),
  })

  return (
    <div className="grid gap-6">
      <Card>
        <CardContent className="space-y-4 pt-6">
          <FileDropzone
            value={files}
            onChange={(f) => {
              setFiles(f)
              run.reset()
            }}
            accept="application/pdf,.pdf"
            multiple={false}
            maxSizeMb={100}
            hint="PDF (con texto — no escaneado)"
            disabled={run.isPending}
          />
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1">
              <Label htmlFor="extract-instruction">Qué extraer</Label>
              <Input
                id="extract-instruction"
                placeholder="Cada producto o línea de pedido"
                value={instruction}
                onChange={(e) => setInstruction(e.target.value)}
                disabled={run.isPending}
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="extract-fields">Campos (separados por comas)</Label>
              <Input
                id="extract-fields"
                value={fields}
                onChange={(e) => setFields(e.target.value)}
                disabled={run.isPending}
              />
            </div>
          </div>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={redactPii}
              onCheckedChange={(v) => setRedactPii(v === true)}
              disabled={run.isPending}
            />
            Enmascarar datos personales (email, IBAN, DNI/NIE, tarjeta, teléfono) antes de enviar
          </label>
          <Button
            onClick={() => file && run.mutate()}
            disabled={!file || run.isPending}
            className="gap-2"
          >
            {run.isPending ? (
              <><Loader2 className="h-4 w-4 animate-spin" /> Extrayendo…</>
            ) : (
              <><Sparkles className="h-4 w-4" /> Extraer datos</>
            )}
          </Button>
        </CardContent>
      </Card>

      {run.isError && (
        <ErrorBox message={friendlyError(run.error, 'No se pudo extraer del PDF.')} />
      )}

      {run.data && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{run.data.items.length} elementos</CardTitle>
            <CardDescription className="flex flex-wrap items-center gap-2">
              <Badge variant="outline">{run.data.model}</Badge>
              <UsageBadges usage={run.data.usage} />
              {run.data.pii_redactions > 0 && (
                <Badge variant="outline">{run.data.pii_redactions} datos enmascarados</Badge>
              )}
              {run.data.truncated_input && (
                <Badge variant="outline" className="text-amber-700 dark:text-amber-400">
                  documento truncado
                </Badge>
              )}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <ItemsTable items={run.data.items} />
          </CardContent>
        </Card>
      )}
    </div>
  )
}

// ── Page ──────────────────────────────────────────────────────────────────────

/**
 * Sanity-check tool for the LLM-backed endpoints, mirroring ProcessPage for
 * spreadsheets: run one file through /api/ocr or /api/pdf/extract and see
 * the result. Real activity is still tracked in Procesamientos.
 */
export default function OcrPage() {
  return (
    <AuthenticatedLayout title="Probar OCR / IA">
      <Main>
        <div className="grid max-w-5xl flex-1 items-start gap-6 md:gap-8">
          <div>
            <div className="flex items-center gap-2">
              <Sparkles className="h-6 w-6 text-primary" />
              <h2 className="text-2xl font-bold tracking-tight">Probar OCR / IA</h2>
            </div>
            <p className="mt-1 text-muted-foreground">
              Prueba rápida de los endpoints con LLM: transcribe una imagen o extrae datos
              estructurados de un PDF. Requiere <code>OCR_LLM_API_KEY</code> configurada en el
              servidor. Toda la actividad real queda registrada en{' '}
              <a href="/admin/jobs" className="underline underline-offset-2">Procesamientos</a>.
            </p>
          </div>

          <Tabs defaultValue="ocr" className="w-full">
            <TabsList>
              <TabsTrigger value="ocr">OCR (imagen)</TabsTrigger>
              <TabsTrigger value="extract">Extracción (PDF)</TabsTrigger>
            </TabsList>
            <TabsContent value="ocr" className="mt-6">
              <OcrTab />
            </TabsContent>
            <TabsContent value="extract" className="mt-6">
              <ExtractTab />
            </TabsContent>
          </Tabs>
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
