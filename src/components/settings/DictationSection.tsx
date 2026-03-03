import { AppSettings } from '../../types';

interface DictationSectionProps {
  settings: AppSettings;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
}

export function DictationSection({ settings, updateSettings }: DictationSectionProps) {
  return (
    <section className="space-y-4">
      <h3 className="section-title secondary">Mode de dictee</h3>

      <div className="space-y-4">
        <div className="flex gap-2">
          {(['general', 'email', 'code', 'notes'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => updateSettings({ dictation_mode: mode })}
              className={`flex-1 px-3 py-2.5 text-[0.8rem] font-medium rounded-xl border transition-all ${
                settings.dictation_mode === mode
                  ? 'bg-[var(--accent-secondary-soft)] border-[var(--accent-secondary)] text-[var(--accent-secondary)]'
                  : 'bg-[rgba(255,255,255,0.08)] border-[var(--glass-border)] text-[var(--text-muted)] hover:border-[var(--accent-secondary)]'
              }`}
            >
              {mode === 'general' && 'General'}
              {mode === 'email' && 'Email'}
              {mode === 'code' && 'Code'}
              {mode === 'notes' && 'Notes'}
            </button>
          ))}
        </div>

        <label className="checkbox-frost">
          <input
            type="checkbox"
            checked={settings.voice_commands_enabled}
            onChange={(e) => updateSettings({ voice_commands_enabled: e.target.checked })}
          />
          <span className="check-box" />
          <span className="check-label">Commandes vocales activees</span>
        </label>

        <label className="checkbox-frost">
          <input
            type="checkbox"
            checked={settings.system_commands_enabled}
            onChange={(e) => updateSettings({ system_commands_enabled: e.target.checked })}
          />
          <span className="check-box" />
          <span className="check-label">Commandes systeme (volume, screenshot...)</span>
        </label>

        <label className="checkbox-frost">
          <input
            type="checkbox"
            checked={settings.subtitles_enabled}
            onChange={(e) => {
              updateSettings({ subtitles_enabled: e.target.checked });
              if (e.target.checked) {
                import('@tauri-apps/api/core').then(({ invoke }) => invoke('show_subtitles_window').catch(console.error));
              } else {
                import('@tauri-apps/api/core').then(({ invoke }) => invoke('hide_subtitles_window').catch(console.error));
              }
            }}
          />
          <span className="check-box" />
          <span className="check-label">Sous-titres en direct</span>
        </label>

        {settings.subtitles_enabled && (
          <div className="pl-8">
            <label className="text-[0.8rem] text-[var(--text-muted)] mb-1 block">
              Taille du texte : {settings.subtitles_font_size}px
            </label>
            <input
              type="range"
              min="12"
              max="48"
              value={settings.subtitles_font_size}
              onChange={(e) => updateSettings({ subtitles_font_size: parseInt(e.target.value) })}
              className="w-full accent-[var(--accent-primary)]"
            />
          </div>
        )}
      </div>
    </section>
  );
}
