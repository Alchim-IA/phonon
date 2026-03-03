import { AppSettings } from '../../types';

interface OptionsSectionProps {
  settings: AppSettings;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
}

export function OptionsSection({ settings, updateSettings }: OptionsSectionProps) {
  return (
    <section className="space-y-4">
      <h3 className="section-title">Options</h3>

      <div className="space-y-3">
        <div>
          <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Theme</label>
          <div className="flex gap-2">
            {(['system', 'dark', 'light'] as const).map((theme) => (
              <button
                key={theme}
                onClick={() => updateSettings({ theme })}
                className={`flex-1 px-3 py-2.5 text-[0.8rem] font-medium rounded-xl border transition-all ${
                  settings.theme === theme
                    ? 'bg-[var(--accent-secondary-soft)] border-[var(--accent-secondary)] text-[var(--accent-secondary)]'
                    : 'bg-[rgba(255,255,255,0.08)] border-[var(--glass-border)] text-[var(--text-muted)] hover:border-[var(--accent-secondary)]'
                }`}
              >
                {theme === 'system' && 'Systeme'}
                {theme === 'dark' && 'Sombre'}
                {theme === 'light' && 'Clair'}
              </button>
            ))}
          </div>
        </div>

        <label className="checkbox-frost">
          <input
            type="checkbox"
            checked={settings.auto_copy_to_clipboard}
            onChange={(e) => updateSettings({ auto_copy_to_clipboard: e.target.checked })}
          />
          <span className="check-box" />
          <span className="check-label">Copier automatiquement dans le presse-papier</span>
        </label>

        <label className="checkbox-frost">
          <input
            type="checkbox"
            checked={settings.notification_on_complete}
            onChange={(e) => updateSettings({ notification_on_complete: e.target.checked })}
          />
          <span className="check-box" />
          <span className="check-label">Notification a la fin de la transcription</span>
        </label>

        <label className="checkbox-frost">
          <input
            type="checkbox"
            checked={settings.minimize_to_tray}
            onChange={(e) => updateSettings({ minimize_to_tray: e.target.checked })}
          />
          <span className="check-box" />
          <span className="check-label">Minimiser dans la barre systeme</span>
        </label>
      </div>
    </section>
  );
}
