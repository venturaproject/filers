import { SettingLayout } from '@/layouts'
import ContentSection from '../components/content-section'
import ProfileForm from './profile-form'
import { useI18n } from '@/i18n/context'

export default function SettingsProfile() {
  const { t } = useI18n()

  return (
    <SettingLayout title={t('user_profile')}>
      <ContentSection
        title={t('profile')}
        desc={t('profile_description')}
      >
        <ProfileForm />
      </ContentSection>
    </SettingLayout>
  )
}
