import { Card, CardContent } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'

type Cell = string | number | boolean | null

export function PreviewTable({
  columns,
  data,
  totalRows,
  maxRows = 100,
}: {
  columns: string[]
  data: Cell[][]
  totalRows?: number
  maxRows?: number
}) {
  const headers =
    columns.length > 0
      ? columns
      : Array.from({ length: data[0]?.length ?? 0 }, (_, i) => `Col ${i + 1}`)
  const rows = data.slice(0, maxRows)
  const total = totalRows ?? data.length

  return (
    <Card>
      <CardContent className="p-0">
        <div className="max-h-[28rem] overflow-auto">
          <Table>
            <TableHeader>
              <TableRow>
                {headers.map((h, i) => (
                  <TableHead key={i} className="whitespace-nowrap">
                    {h}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((row, i) => (
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
              {total > rows.length && (
                <TableRow>
                  <TableCell
                    colSpan={Math.max(headers.length, 1)}
                    className="py-2 text-center text-xs text-muted-foreground"
                  >
                    Mostrando {rows.length} de {total.toLocaleString('es-ES')} filas
                  </TableCell>
                </TableRow>
              )}
              {data.length === 0 && (
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
