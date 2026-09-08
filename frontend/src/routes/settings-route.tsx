import { useParams } from 'react-router-dom'
import { useAuthStore } from '@/lib/auth'

// Settings pages
import ProfilePage from '@/pages/settings/profile'
import AppearancePage from '@/pages/settings/appearance'
import NotificationsPage from '@/pages/settings/notifications'
import DisplayPage from '@/pages/settings/display'
import AccountPage from '@/pages/settings/account'

export default function SettingsRoute() {
  const { section } = useParams()
  const user = useAuthStore((s) => s.user)

  switch (section) {
    case 'appearance': return <AppearancePage />
    case 'notifications': return <NotificationsPage />
    case 'display': return <DisplayPage />
    case 'account': return <AccountPage />
    default: return <ProfilePage />
  }
}
