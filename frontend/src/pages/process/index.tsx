import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
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
import { filesApi, type ParsedFile, type ParsedSheet } from '@/services/files-api'

function SheetTable({ sheet }: { sheet: ParsedSheet }) {
  const headers = sheet.rows.length > 0 ? Object.keys(sheet.rows[0]) : []
  const preview = sheet.rows.slice(0, 100)

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm flex items-center gap-2">
          {sheet.name}
          <span className="text-muted-foreground font-normal">
            {sheet.row_count} filas · {sheet.col_count} columnas
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent className="p-0">
        <div className="overflow-x-auto max-h-96">
          <Table>
            <TableHeader>
              <TableRow>
                {headers.map((h) => (
                  <TableHead key={h} className="whitespace-nowrap">{h}</TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {preview.map((row, i) => (
                <TableRow key={i}>
                  {headers.map((h) => (
                    <TableCell key={h} className="whitespace-nowrap text-sm">
                      {row[h] == null ? <span className="text-muted-foreground">—</span> : String(row[h])}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
              {sheet.row_count > 100 && (
                <TableRow>
                  <TableCell colSpan={headers.length} className="text-center text-muted-foreground text-xs py-2">
                    Mostrando 100 de {sheet.row_count} filas
                  </TableCell>
                </TableRow>
              )}
              {sheet.row_count === 0 && (
                <TableRow>
                  <TableCell colSpan={Math.max(headers.length, 1)} className="text-center text-muted-foreground">
                    Hoja vacía
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
  const [file, setFile] = useState<File | null>(null)

  const process = useMutation({
    mutationFn: () => filesApi.process(file as File, { header_row: true }),
  })

  const result: ParsedFile | undefined = process.data

  return (
    <AuthenticatedLayout title="Procesar archivo">
      <Main>
        <div className="grid flex-1 items-start gap-6 md:gap-8 max-w-5xl">
          <div>
            <div className="flex items-center gap-2">
              <FileSpreadsheet className="h-6 w-6 text-primary" />
              <h2 className="text-2xl font-bold tracking-tight">Procesar archivo</h2>
            </div>
            <p className="text-muted-foreground mt-1">
              Sube un Excel (.xlsx, .xls) o CSV y Filers devolverá el contenido en JSON estructurado.
              Para procesar grandes volúmenes usa los{' '}
              <a href="/admin/jobs" className="underline underline-offset-2">trabajos batch</a>.
            </p>
          </div>

          <Card>
            <CardContent className="pt-6">
              <form
                className="flex flex-col gap-4"
                onSubmit={(e) => {
                  e.preventDefault()
                  if (file) process.mutate()
                }}
              >
                <div className="grid gap-2">
                  <label className="text-sm font-medium">
                    Archivo (XLSX, XLS, ODS, CSV, TSV)
                  </label>
                  <Input
                    type="file"
                    accept=".xlsx,.xls,.ods,.csv,.tsv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.ms-excel,text/csv"
                    onChange={(e) => {
                      setFile(e.target.files?.[0] ?? null)
                      process.reset()
                    }}
                  />
                </div>

                <div>
                  <Button type="submit" disabled={!file || process.isPending} className="gap-2">
                    {process.isPending ? (
                      <><Loader2 className="h-4 w-4 animate-spin" /> Procesando…</>
                    ) : (
                      <><Upload className="h-4 w-4" /> Procesar archivo</>
                    )}
                  </Button>
                </div>
              </form>
            </CardContent>
          </Card>

          {process.isError && (
            <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/30 dark:text-red-400">
              <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
              <span>
                {(process.error as any)?.response?.data?.error ?? 'No se pudo procesar el archivo.'}
              </span>
            </div>
          )}

          {result && (
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Resultado</CardTitle>
                  <CardDescription className="flex flex-wrap gap-2 items-center">
                    <Badge variant="outline">{result.format.toUpperCase()}</Badge>
                    <span>{result.stats.row_count} filas</span>
                    <span>·</span>
                    <span>{result.stats.col_count} columnas</span>
                    {result.stats.sheet_count > 1 && (
                      <><span>·</span><span>{result.stats.sheet_count} hojas</span></>
                    )}
                    <span>·</span>
                    <span>{result.stats.processing_ms} ms</span>
                  </CardDescription>
                </CardHeader>
              </Card>

              {result.sheets.map((sheet) => (
                <SheetTable key={sheet.name} sheet={sheet} />
              ))}
            </>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
