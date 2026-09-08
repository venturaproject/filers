import { useEffect, useRef, useState } from 'react'

function getStorageKey(storageKey: string) {
  return `column-order:${storageKey}`
}

function getVisibilityKey(storageKey: string) {
  return `column-visibility:${storageKey}`
}

function readStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}

/**
 * Gestiona el orden y visibilidad de columnas con persistencia en localStorage.
 *
 * La clave de almacenamiento se deriva automáticamente de la ruta actual,
 * por lo que cada página tiene su propio estado guardado sin configuración extra.
 *
 * @param initialIds - Orden inicial de columnas (usado si no hay nada en localStorage)
 */
export function useColumnReorder(initialIds: string[]) {
  const storageKey = window.location.pathname
  const [columnOrder, setColumnOrder] = useState<string[]>(() =>
    readStorage(getStorageKey(storageKey), initialIds),
  )
  const [columnVisibility, setColumnVisibility] = useState<Record<string, boolean>>(() =>
    readStorage(getVisibilityKey(storageKey), {}),
  )
  const dragColumnId = useRef<string | null>(null)

  // Persistir orden cuando cambia
  useEffect(() => {
    try {
      localStorage.setItem(getStorageKey(storageKey), JSON.stringify(columnOrder))
    } catch {
      // localStorage puede estar lleno o bloqueado en modo incógnito
    }
  }, [columnOrder, storageKey])

  // Persistir visibilidad cuando cambia
  useEffect(() => {
    try {
      localStorage.setItem(getVisibilityKey(storageKey), JSON.stringify(columnVisibility))
    } catch {
      // localStorage puede estar lleno o bloqueado en modo incógnito
    }
  }, [columnVisibility, storageKey])

  const handleDragStart = (id: string) => {
    dragColumnId.current = id
  }

  const handleDrop = (targetId: string) => {
    if (!dragColumnId.current || dragColumnId.current === targetId) return
    setColumnOrder((prev) => {
      const next = [...prev]
      const from = next.indexOf(dragColumnId.current!)
      const to = next.indexOf(targetId)
      next.splice(from, 1)
      next.splice(to, 0, dragColumnId.current!)
      return next
    })
    dragColumnId.current = null
  }

  return {
    columnOrder,
    columnVisibility,
    setColumnVisibility,
    handleDragStart,
    handleDrop,
  }
}
