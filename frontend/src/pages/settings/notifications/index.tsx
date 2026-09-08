import { SettingLayout } from "@/layouts"
import ContentSection from '../components/content-section'
import { NotificationsForm } from './notifications-form'
import { useI18n } from '@/i18n/context'

export default function SettingsNotifications() {
  const { t } = useI18n()
  
  return (
      <SettingLayout title={t('notifications_settings')}>
        <ContentSection
          title={t('notifications')}
          desc={t('notifications_description')}
        >
          <NotificationsForm />
        </ContentSection>
      </SettingLayout>
  )
}
