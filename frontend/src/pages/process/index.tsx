import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
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
import { AlertCircle, FileSpreadsheet, Loader2, Upload } from 'lucide-react'
import { FileDropzone } from '@/components/file-dropzone'
import { filesApi, type ParsedFile } from '@/services/files-api'

const PREVIEW_ROWS = 100

function ResultTable({ result }: { result: ParsedFile }) {
  const headers =
    result.columns.length > 0
      ? result.columns
      : Array.from({ length: result.data[0]?.length ?? 0 }, (_, i) => `Col ${i + 1}`)
  const preview = result.data.slice(0, PREVIEW_ROWS)

  return (
    <Card>
      <CardContent className="p-0">
        <div className="max-h-[28rem] overflow-auto">
          <Table>
            <TableHeader>
              <TableRow>
                {headers.map((h, i) => (
                  <TableHead key={i} className="whitespace-nowrap">{h}</TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {preview.map((row, i) => (
                <TableRow key={i}>
                  {headers.map((_, c) => (
                    <TableCell key={c} className="whitespace-nowrap text-sm">
                      {row[c] == null ? (
                        <span className="text-muted-foreground">—</span>
                      ) : (
                        String(row[c])
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
              {result.stats.total_rows > preview.length && (
                <TableRow>
                  <TableCell
                    colSpan={Math.max(headers.length, 1)}
                    className="py-2 text-center text-xs text-muted-foreground"
                  >
                    Mostrando {preview.length} de {result.stats.total_rows.toLocaleString('es-ES')} filas
                  </TableCell>
                </TableRow>
              )}
              {result.data.length === 0 && (
                <TableRow>
                  <TableCell
                    colSpan={Math.max(headers.length, 1)}
                    className="text-center text-muted-foreground"
                  >
                    Sin datos
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  )
}

export default function ProcessPage() {
  const [files, setFiles] = useState<File[]>([])
  const file = files[0] ?? null

  const process = useMutation({
    mutationFn: () => filesApi.process(file as File, { has_headers: true, max_rows: PREVIEW_ROWS }),
  })

  const result = process.data

  return (
    <AuthenticatedLayout title="Procesar archivo">
      <Main>
        <div className="grid max-w-5xl flex-1 items-start gap-6 md:gap-8">
          <div>
            <div className="flex items-center gap-2">
              <FileSpreadsheet className="h-6 w-6 text-primary" />
              <h2 className="text-2xl font-bold tracking-tight">Procesar archivo</h2>
            </div>
            <p className="mt-1 text-muted-foreground">
              Sube un Excel (.xlsx, .xls, .ods) o CSV y Filers devuelve el contenido en JSON
              estructurado. Para varios archivos usa los{' '}
              <a href="/admin/jobs" className="underline underline-offset-2">Procesamientos</a>.
            </p>
          </div>

          <Card>
            <CardContent className="space-y-4 pt-6">
              <FileDropzone
                value={files}
                onChange={(f) => {
                  setFiles(f)
                  process.reset()
                }}
                multiple={false}
                maxSizeMb={100}
                disabled={process.isPending}
              />
              <Button
                onClick={() => file && process.mutate()}
                disabled={!file || process.isPending}
                className="gap-2"
              >
                {process.isPending ? (
                  <><Loader2 className="h-4 w-4 animate-spin" /> Procesando…</>
                ) : (
                  <><Upload className="h-4 w-4" /> Procesar archivo</>
                )}
              </Button>
            </CardContent>
          </Card>

          {process.isError && (
            <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/30 dark:text-red-400">
              <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
              <span>
                {(process.error as { response?: { data?: { error?: string } } })?.response?.data
                  ?.error ?? 'No se pudo procesar el archivo.'}
              </span>
            </div>
          )}

          {result && (
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Resultado</CardTitle>
                  <CardDescription className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">{result.format.toUpperCase()}</Badge>
                    <span>{result.stats.total_rows.toLocaleString('es-ES')} filas</span>
                    <span>·</span>
                    <span>{result.stats.columns} columnas</span>
                    <span>·</span>
                    <span>{result.stats.elapsed_ms} ms</span>
                    {result.errors.length > 0 && (
                      <>
                        <span>·</span>
                        <span className="text-destructive">{result.errors.length} errores</span>
                      </>
                    )}
                  </CardDescription>
                </CardHeader>
              </Card>

              <ResultTable result={result} />
            </>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
