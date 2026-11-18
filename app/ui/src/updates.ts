// The Help tab's Check for Updates: the pieces that need no window (HelpView.tsx; docs/UPDATES.md). The daily-check
// setting lives in the window's own storage, which an installed copy keeps between runs.

export interface UpdatePrefs {
  daily: boolean;
  lastCheck: string;
}

const KEY = 'prismsurvey.updates';

export function readPrefs(): UpdatePrefs {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<UpdatePrefs>;
    return { daily: v.daily === true, lastCheck: typeof v.lastCheck === 'string' ? v.lastCheck : '' };
  } catch {
    return { daily: false, lastCheck: '' };
  }
}

export function writePrefs(p: UpdatePrefs): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    /* the setting is not kept this time; nothing else depends on it */
  }
}

/** How far a download has got, as a whole percent, or null when the size isn't known. */
export function progressPercent(received: number, total: number | null | undefined): number | null {
  if (!total || total <= 0) return null;
  return Math.min(100, Math.floor((100 * received) / total));
}

/** Whether the once-a-day check is due: turned on, and never checked or last checked a day or more ago. */
export function dailyCheckDue(on: boolean, lastCheck: string, now: Date = new Date()): boolean {
  if (!on) return false;
  const last = Date.parse(lastCheck);
  return Number.isNaN(last) || now.getTime() - last >= 24 * 60 * 60 * 1000;
}
