import { create } from 'zustand'

interface FlashState {
  success?: string
  error?: string
  setFlash: (flash: { success?: string; error?: string }) => void
  clearFlash: () => void
}

export const useFlashStore = create<FlashState>((set) => ({
  success: undefined,
  error: undefined,
  setFlash: (flash) => set(flash),
  clearFlash: () => set({ success: undefined, error: undefined }),
}))
