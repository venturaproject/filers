import { useLocation } from 'react-router-dom'
import ProcessPage from '@/pages/process/index'
import JobsPage from '@/pages/process/jobs'
import ComparePage from '@/pages/process/compare'
import PipelinePage from '@/pages/process/pipeline'

export default function ProcessRoute() {
  const { pathname } = useLocation()
  if (pathname.startsWith('/admin/jobs')) return <JobsPage />
  if (pathname.startsWith('/admin/compare')) return <ComparePage />
  if (pathname.startsWith('/admin/pipeline')) return <PipelinePage />
  return <ProcessPage />
}
