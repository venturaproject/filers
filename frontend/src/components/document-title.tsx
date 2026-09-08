import { useEffect } from 'react'
import { APP_NAME } from '@/config/env'

export function DocumentTitle({ title }: { title?: string }) {
  useEffect(() => {
    document.title = title ? `${title} - ${APP_NAME}` : APP_NAME
  }, [title])

  return null
}
