import { useCallback, useRef, useState } from 'react'
import { FileSpreadsheet, UploadCloud, X } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

interface FileDropzoneProps {
  value: File[]
  onChange: (files: File[]) => void
  accept?: string
  multiple?: boolean
  maxSizeMb?: number
  disabled?: boolean
  hint?: string
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(1)} MB`
}

/**
 * Drag-and-drop file input in the shadcn style: a dashed dropzone that also
 * opens the native picker on click, plus a removable list of selected files.
 */
export function FileDropzone({
  value,
  onChange,
  accept = '.xlsx,.xls,.ods,.csv,.tsv',
  multiple = true,
  maxSizeMb = 100,
  disabled = false,
  hint = 'XLSX · XLS · ODS · CSV · TSV',
}: FileDropzoneProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  const [dragActive, setDragActive] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const addFiles = useCallback(
    (incoming: FileList | null) => {
      if (!incoming || incoming.length === 0) return
      const list = Array.from(incoming)
      const tooBig = list.find((f) => f.size > maxSizeMb * 1024 * 1024)
      if (tooBig) {
        setError(`"${tooBig.name}" supera el límite de ${maxSizeMb} MB`)
        return
      }
      setError(null)
      onChange(multiple ? [...value, ...list] : list.slice(0, 1))
    },
    [value, onChange, multiple, maxSizeMb],
  )

  const open = () => !disabled && inputRef.current?.click()

  return (
    <div className="space-y-3">
      <div
        role="button"
        tabIndex={disabled ? -1 : 0}
        aria-disabled={disabled}
        onClick={open}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            open()
          }
        }}
        onDragOver={(e) => {
          e.preventDefault()
          if (!disabled) setDragActive(true)
        }}
        onDragLeave={(e) => {
          e.preventDefault()
          setDragActive(false)
        }}
        onDrop={(e) => {
          e.preventDefault()
          setDragActive(false)
          if (!disabled) addFiles(e.dataTransfer.files)
        }}
        className={cn(
          'flex flex-col items-center justify-center gap-2 rounded-lg border-2 border-dashed px-6 py-10 text-center transition-colors',
          'cursor-pointer hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
          dragActive ? 'border-primary bg-primary/5' : 'border-input',
          disabled && 'pointer-events-none opacity-60',
        )}
      >
        <UploadCloud
          className={cn('h-8 w-8', dragActive ? 'text-primary' : 'text-muted-foreground')}
        />
        <div className="text-sm">
          <span className="font-medium text-foreground">Arrastra archivos aquí</span>{' '}
          o haz clic para elegir
        </div>
        <p className="text-xs text-muted-foreground">
          {hint} — hasta {maxSizeMb} MB
        </p>
        <input
          ref={inputRef}
          type="file"
          className="hidden"
          accept={accept}
          multiple={multiple}
          disabled={disabled}
          onChange={(e) => {
            addFiles(e.target.files)
            e.target.value = ''
          }}
        />
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}

      {value.length > 0 && (
        <ul className="space-y-1.5">
          {value.map((f, i) => (
            <li
              key={`${f.name}-${i}`}
              className="flex items-center gap-2 rounded-md border bg-muted/30 px-3 py-2 text-sm"
            >
              <FileSpreadsheet className="h-4 w-4 shrink-0 text-muted-foreground" />
              <span className="flex-1 truncate">{f.name}</span>
              <span className="shrink-0 text-xs text-muted-foreground">{formatBytes(f.size)}</span>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-6 w-6 shrink-0"
                disabled={disabled}
                onClick={() => onChange(value.filter((_, j) => j !== i))}
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
