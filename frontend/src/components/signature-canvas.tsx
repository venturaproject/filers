import { useRef, useEffect, useState, useCallback } from 'react'
import SignaturePad from 'signature_pad'
import { Button } from '@/components/ui/button'
import { Eraser, Undo2 } from 'lucide-react'

interface SignatureCanvasProps {
  onSignatureChange: (dataUri: string | null) => void
  width?: number
  height?: number
}

export function SignatureCanvas({ onSignatureChange, width = 600, height = 200 }: SignatureCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const signaturePadRef = useRef<SignaturePad | null>(null)
  const [isEmpty, setIsEmpty] = useState(true)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const dpr = window.devicePixelRatio || 1
    canvas.width = width * dpr
    canvas.height = height * dpr
    canvas.style.width = `${width}px`
    canvas.style.height = `${height}px`

    const ctx = canvas.getContext('2d')
    if (ctx) {
      ctx.scale(dpr, dpr)
    }

    const pad = new SignaturePad(canvas, {
      backgroundColor: 'rgba(255, 255, 255, 0)',
      penColor: 'rgb(17, 24, 39)',
      minWidth: 1.4,
      maxWidth: 3.2,
      velocityFilterWeight: 0.9,
    })

    pad.addEventListener('endStroke', () => {
      setIsEmpty(pad.isEmpty())
      if (!pad.isEmpty()) {
        onSignatureChange(pad.toDataURL('image/png'))
      }
    })

    signaturePadRef.current = pad

    return () => {
      pad.off()
    }
  }, [width, height, onSignatureChange])

  const handleUndo = useCallback(() => {
    const pad = signaturePadRef.current
    if (!pad) return
    const data = pad.toData()
    if (data.length > 0) {
      data.pop()
      pad.fromData(data)
      setIsEmpty(pad.isEmpty())
      if (pad.isEmpty()) {
        onSignatureChange(null)
      } else {
        onSignatureChange(pad.toDataURL('image/png'))
      }
    }
  }, [onSignatureChange])

  const handleClear = useCallback(() => {
    signaturePadRef.current?.clear()
    setIsEmpty(true)
    onSignatureChange(null)
  }, [onSignatureChange])

  return (
    <div className="space-y-2">
      <div className="rounded-lg border-2 border-dashed border-gray-300 bg-white">
        <canvas
          ref={canvasRef}
          className="w-full cursor-crosshair touch-none"
          style={{ width: '100%', maxWidth: width, height }}
        />
      </div>
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {isEmpty ? 'Firma en el recuadro de arriba' : 'Firma capturada'}
        </p>
        <div className="flex gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleUndo}
            disabled={isEmpty}
          >
            <Undo2 className="mr-1 h-3 w-3" />
            Deshacer
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleClear}
            disabled={isEmpty}
          >
            <Eraser className="mr-1 h-3 w-3" />
            Limpiar
          </Button>
        </div>
      </div>
    </div>
  )
}
