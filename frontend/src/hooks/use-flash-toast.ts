// Flash messages in the SPA context come from API responses, not from Inertia
// page props. Pages that need toasts should call toast() directly after their
// axios calls. This hook is kept as a no-op so callers don't break while the
// remaining Inertia pages are migrated.
export function useFlashToast() {}
