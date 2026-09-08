import { SettingLayout } from '@/layouts'
import ContentSection from '../components/content-section'
import { AppearanceForm } from './appearance-form'
import { useI18n } from '@/i18n/context'

export default function SettingsAppearance() {
  const { t } = useI18n()
  
  return (
    <SettingLayout title={t('appearance_settings')}>
      <ContentSection
        title={t('appearance')}
        desc={t('appearance_description')}
      >
        <AppearanceForm />
      </ContentSection>
    </SettingLayout>
  )
}
