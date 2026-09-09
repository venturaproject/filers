import { useLocation } from 'react-router-dom'
import ProcessPage from '@/pages/process/index'
import JobsPage from '@/pages/process/jobs'

export default function ProcessRoute() {
  const { pathname } = useLocation()
  if (pathname.startsWith('/admin/jobs')) return <JobsPage />
  return <ProcessPage />
}
