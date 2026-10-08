import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { useTranslation } from 'react-i18next'
import { If } from 'react-if-lite'
import { PanelHeader } from './panel-header'
import { PanelState } from './panel-state'

interface BundledCore { version: string, path: string }

export function ConfigCore() {
  const { t } = useTranslation()
  const { data, isLoading, error } = useQuery({
    queryKey: ['cores'],
    queryFn: () => invoke<BundledCore[]>('get_cores'),
  })
  const { data: latestVersion } = useQuery({
    queryKey: ['core-latest-version'],
    queryFn: () => invoke<string | null>('get_latest_core_version').catch(() => null),
    staleTime: 5 * 60 * 1000,
    retry: false,
  })
  return (
    <div className="space-y-3">
      <PanelHeader title={t('core.title')} description={t('core.bundled_hint')} />
      <PanelState loading={isLoading} error={error?.message ?? ''}>
        <p className="text-sm">{t('core.bundled_version', { version: data?.[0]?.version || '—' })}</p>
        <If cond={latestVersion != null}>
          <p className="text-sm">{t('core.latest_version', { version: latestVersion })}</p>
        </If>
        <p className="break-all text-xs text-muted">{data?.[0]?.path}</p>
      </PanelState>
    </div>
  )
}
