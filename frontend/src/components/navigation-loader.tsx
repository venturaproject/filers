import { useEffect, useRef, useState } from 'react'
import { useNavigation } from 'react-router-dom'
import { useIsFetching, useIsMutating } from '@tanstack/react-query'

const loaderCss = `
  .navigation-loader {
    position: fixed;
    inset: 0 0 auto 0;
    height: 3px;
    z-index: 9999;
    pointer-events: none;
    opacity: 0;
    transition: opacity 200ms ease;
  }

  .navigation-loader.is-visible {
    opacity: 1;
  }

  .navigation-loader__bar {
    height: 100%;
    background: #4b5563;
    transform-origin: left center;
    box-shadow: 0 0 10px rgba(75, 85, 99, 0.45);
    transition: transform 180ms ease-out;
  }

  .navigation-loader__peg {
    display: block;
    position: absolute;
    right: 0;
    width: 100px;
    height: 100%;
    opacity: 1;
    transform: rotate(3deg) translate(0px, -4px);
    box-shadow:
      0 0 10px #4b5563,
      0 0 5px #4b5563;
  }
`

export function NavigationLoader() {
  const navigation = useNavigation()
  const activeQueries = useIsFetching()
  const activeMutations = useIsMutating()
  const [mounted, setMounted] = useState(false)
  const [visible, setVisible] = useState(false)
  const [progress, setProgress] = useState(0)
  const isLoading =
    navigation.state === 'loading' ||
    navigation.state === 'submitting' ||
    activeQueries > 0 ||
    activeMutations > 0
  const timerRef = useRef<number | null>(null)

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current)
      timerRef.current = null
    }
  }

  useEffect(() => {
    setMounted(true)
    return () => clearTimer()
  }, [])

  useEffect(() => {
    if (isLoading) {
      clearTimer()
      setVisible(true)
      setProgress((current) => (current > 0 ? current : 8))

      timerRef.current = window.setInterval(() => {
        setProgress((current) => {
          if (current >= 92) return current
          if (current < 25) return current + 12
          if (current < 55) return current + 7
          if (current < 80) return current + 3
          return current + 1
        })
      }, 180)

      return
    }

    clearTimer()

    if (!visible) return

    setProgress(100)

    const hideTimeout = window.setTimeout(() => {
      setVisible(false)
      setProgress(0)
    }, 220)

    return () => window.clearTimeout(hideTimeout)
  }, [isLoading, visible])

  return (
    <>
      {mounted ? <style>{loaderCss}</style> : null}
      <div
        aria-hidden="true"
        className={`navigation-loader${visible ? ' is-visible' : ''}`}
      >
        <div
          className="navigation-loader__bar"
          style={{ transform: `scaleX(${progress / 100})` }}
        >
          <span className="navigation-loader__peg" />
        </div>
      </div>
    </>
  )
}
