// Canal oficial de distribuição do instalador Windows.
// O arquivo de nome fixo é sobrescrito a cada release (scripts/publish-update.ps1),
// então este link nunca muda entre versões.
export const DESKTOP_DOWNLOAD_URL =
  (import.meta.env.VITE_DESKTOP_DOWNLOAD_URL as string | undefined)?.trim() ||
  "https://sqwpkwsjbbkylbdpjrlu.supabase.co/storage/v1/object/public/desktop-releases/CommerceCTRL-Setup.exe";
