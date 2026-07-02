const LOCKED_KEY = "app_lock_manually_locked";

export function readManualLocked(): boolean {
  try {
    return localStorage.getItem(LOCKED_KEY) === "true";
  } catch {
    return false;
  }
}

export function writeManualLocked(locked: boolean) {
  try {
    localStorage.setItem(LOCKED_KEY, locked ? "true" : "false");
  } catch {
    /* ignore */
  }
}

export function clearManualLocked() {
  writeManualLocked(false);
}
