import { useWatch } from '@hairy/react-lib'
import { useOverlay } from '@overlastic/react'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useStore } from 'valtio-define'
import { DesktopUpdateDialog } from '@/components/desktop-update-dialog'
import { store } from '@/store'
import { toast } from '@/utils/toast'

/** 桌面端自更新轮询间隔：Rust 侧不再缓存，改为低频轮询以免触发 GitHub 未认证限流（60 次/小时/IP） */
const POLL_INTERVAL = 10 * 60_000

/**
 * 桌面端「发现新版本」提示：后台低频轮询（见 POLL_INTERVAL，Rust 侧不缓存、
 * 每次实时查询），发现新版本时在右下角弹 toast。
 * 用户关闭后记住该版本，本次会话不再弹出；新版本出现仍会再次提示。
 * 应用启动时首次检查发现的版本只更新菜单指示，不弹 toast。
 */
export function DesktopUpdater() {
  const { t } = useTranslation()
  const { updateInfo, dismissedTag, downloading } = useStore(store.desktopUpdater)
  const openUpdateDialog = useOverlay(DesktopUpdateDialog)
  const startupCheckedRef = useRef(false)
  const startupTagRef = useRef<string | undefined>(undefined)

  // 低频静默检查新版本（实时查询，无本地缓存）；后台失败静默，不打扰用户
  useEffect(() => {
    void store.desktopUpdater.check()
      .then((info) => {
        startupTagRef.current = info?.tag
      })
      .catch(() => {})
      .finally(() => {
        startupCheckedRef.current = true
      })
    const timer = setInterval(() => {
      void store.desktopUpdater.check().catch(() => {})
    }, POLL_INTERVAL)
    return () => clearInterval(timer)
  }, [])

  useWatch([updateInfo, dismissedTag, downloading], () => {
    if (!updateInfo || downloading)
      return
    // 安装包已下载（用户已发起更新）→ 不再重复弹「立即更新」toast
    if (updateInfo.downloaded)
      return
    // 用户已关闭过该版本提示 → 不再弹出
    if (updateInfo.tag === dismissedTag)
      return
    // 启动检查尚未结束或发现的就是启动时的版本 → 不弹出
    if (!startupCheckedRef.current || updateInfo.tag === startupTagRef.current)
      return
    toast(t('update.available', { tag: updateInfo.tag }), {
      actionProps: {
        children: t('update.now'),
        onPress: () => {
          toast.clear()
          // 打开更新对话框并开始下载，对话框内展示下载进度（下载完成自动关闭）
          openUpdateDialog()
          void store.desktopUpdater.downloadAndOpen()
        },
        variant: 'tertiary',
      },
      timeout: 0,
      placement: 'bottom end',
      description: t('update.desktop_new'),
      variant: 'default',
      onClose: () => store.desktopUpdater.dismissToast(),
    })
  }, { immediate: true })

  return null
}
