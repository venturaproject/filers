import { SettingLayout } from '@/layouts'
import ContentSection from '../components/content-section'
import { DisplayForm } from './display-form'
import { useI18n } from '@/i18n/context'

export default function SettingsDisplay() {
  const { t } = useI18n()
  
  return (
  <SettingLayout title={t('display_settings')}>
    <ContentSection
      title={t('display')}
      desc={t('display_settings_description')}
    >
      <DisplayForm />
    </ContentSection>
  </SettingLayout>
  )
}
