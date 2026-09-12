import { useLocation } from 'react-router-dom'
import ProcessPage from '@/pages/process/index'
import JobsPage from '@/pages/process/jobs'
import OcrPage from '@/pages/process/ocr'

export default function ProcessRoute() {
  const { pathname } = useLocation()
  if (pathname.startsWith('/admin/jobs')) return <JobsPage />
  if (pathname.startsWith('/admin/ocr')) return <OcrPage />
  return <ProcessPage />
}
