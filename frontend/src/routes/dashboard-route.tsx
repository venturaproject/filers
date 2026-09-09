import { useQuery } from '@tanstack/react-query'
import { axios } from '@/lib/axios'
import { API_ENDPOINTS } from '@/config'
import Dashboard, { type DashboardData, EMPTY_DASHBOARD } from '@/pages/dashboard/index'

export default function DashboardRoute() {
  const { data } = useQuery({
    queryKey: ['dashboard'],
    queryFn: () => axios.get<DashboardData>(API_ENDPOINTS.dashboard).then((r) => r.data),
    refetchInterval: 15000,
  })

  return <Dashboard data={data ?? EMPTY_DASHBOARD} />
}
