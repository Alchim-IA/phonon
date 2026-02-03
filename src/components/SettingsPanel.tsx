import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore } from '../stores/settingsStore';
import { HotkeyInput } from './HotkeyInput';
import { ModelSize, ModelInfo, DownloadProgress } from '../types';

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
  const { settings, devices, dictionary, loadSettings, loadDevices, loadDictionary, updateSettings, addWord, removeWord } = useSettingsStore();
  const [newWord, setNewWord] = useState('');
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<ModelSize | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
      loadDevices();
      loadDictionary();
      loadModels();
    }
  }, [isOpen, loadSettings, loadDevices, loadDictionary]);

  useEffect(() => {
    const unlistenProgress = listen<DownloadProgress>('model-download-progress', (event) => {
      setDownloadProgress(event.payload);
    });

    const unlistenComplete = listen<ModelSize>('model-download-complete', () => {
      setDownloading(null);
      setDownloadProgress(null);
      loadModels();
    });

    return () => {
      unlistenProgress.then(fn => fn());
      unlistenComplete.then(fn => fn());
    };
  }, []);

  const loadModels = async () => {
    try {
      const result = await invoke<ModelInfo[]>('get_available_models');
      setModels(result);
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  };

  const handleDownloadModel = async (size: ModelSize) => {
    setDownloading(size);
    setDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    try {
      await invoke('download_model', { size });
    } catch (e) {
      console.error('Download failed:', e);
      setDownloading(null);
      setDownloadProgress(null);
    }
  };

  const handleSwitchModel = async (size: ModelSize) => {
    try {
      await invoke('switch_model', { size });
      await loadSettings();
    } catch (e) {
      console.error('Switch failed:', e);
    }
  };

  const handleAddWord = async () => {
    if (newWord.trim()) {
      await addWord(newWord.trim());
      setNewWord('');
    }
  };

  if (!isOpen || !settings) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="settings-panel relative w-full max-w-md h-full bg-[var(--bg-panel)] border-l border-[var(--border-subtle)] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex-shrink-0 px-5 py-4 bg-[var(--bg-elevated)] border-b border-[var(--border-subtle)] flex justify-between items-center">
          <div className="flex items-center gap-3">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="1.5">
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
            <h2 className="font-display font-semibold text-[var(--text-primary)] tracking-tight">
              Configuration
            </h2>
          </div>
          <button
            onClick={onClose}
            className="w-8 h-8 flex items-center justify-center rounded border border-[var(--border-subtle)] hover:border-[var(--accent-red)] hover:text-[var(--accent-red)] transition-colors"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 space-y-6 scrollbar-thin">
          {/* Audio Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-cyan)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-cyan)]/30" />
              Audio
            </h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Microphone
                </span>
                <select
                  value={settings.microphone_id || ''}
                  onChange={(e) => updateSettings({ microphone_id: e.target.value || null })}
                  className="select-field w-full"
                >
                  <option value="">Par defaut</option>
                  {devices.map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.name} {device.is_default ? '(defaut)' : ''}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </section>

          {/* Engine Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-green)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-green)]/30" />
              Moteur Whisper
            </h3>

            <div className="space-y-2">
              {models.map((model) => (
                <div
                  key={model.size}
                  className={`flex items-center justify-between p-3 border rounded ${
                    settings.whisper_model === model.size
                      ? 'border-[var(--accent-green)] bg-[var(--accent-green)]/5'
                      : 'border-[var(--border-subtle)]'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <div className={`w-2 h-2 rounded-full ${
                      settings.whisper_model === model.size
                        ? 'bg-[var(--accent-green)]'
                        : 'bg-[var(--border-subtle)]'
                    }`} />
                    <div>
                      <div className="text-sm text-[var(--text-primary)]">
                        {model.display_name}
                      </div>
                      {model.size === 'small' && (
                        <div className="text-[0.6rem] text-[var(--accent-cyan)]">Recommande</div>
                      )}
                    </div>
                  </div>

                  {downloading === model.size ? (
                    <div className="flex items-center gap-2">
                      <div className="w-20 h-1.5 bg-[var(--bg-elevated)] rounded overflow-hidden">
                        <div
                          className="h-full bg-[var(--accent-cyan)] transition-all"
                          style={{ width: `${downloadProgress?.percent || 0}%` }}
                        />
                      </div>
                      <span className="text-[0.6rem] text-[var(--text-muted)] w-10">
                        {Math.round(downloadProgress?.percent || 0)}%
                      </span>
                    </div>
                  ) : model.available ? (
                    settings.whisper_model === model.size ? (
                      <span className="text-[0.6rem] text-[var(--accent-green)] uppercase">Actif</span>
                    ) : (
                      <button
                        onClick={() => handleSwitchModel(model.size)}
                        className="text-[0.6rem] text-[var(--accent-cyan)] hover:underline uppercase"
                      >
                        Utiliser
                      </button>
                    )
                  ) : (
                    <button
                      onClick={() => handleDownloadModel(model.size)}
                      className="text-[0.6rem] text-[var(--text-muted)] hover:text-[var(--accent-cyan)] uppercase flex items-center gap-1"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="7 10 12 15 17 10" />
                        <line x1="12" y1="15" x2="12" y2="3" />
                      </svg>
                      Telecharger
                    </button>
                  )}
                </div>
              ))}
            </div>
          </section>

          {/* Transcription Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-magenta)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-magenta)]/30" />
              Transcription
            </h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Langue
                </span>
                <select
                  value={settings.auto_detect_language ? 'auto' : settings.transcription_language}
                  onChange={(e) => {
                    if (e.target.value === 'auto') {
                      updateSettings({ auto_detect_language: true });
                    } else {
                      updateSettings({
                        transcription_language: e.target.value,
                        auto_detect_language: false
                      });
                    }
                  }}
                  className="select-field w-full"
                >
                  <option value="auto">Automatique (detection)</option>
                  <option value="fr">Francais</option>
                  <option value="en">English</option>
                  <option value="de">Deutsch</option>
                  <option value="es">Espanol</option>
                  <option value="it">Italiano</option>
                  <option value="pt">Portugues</option>
                  <option value="nl">Nederlands</option>
                  <option value="pl">Polski</option>
                  <option value="ru">Russkiy</option>
                  <option value="ja">Nihongo</option>
                  <option value="zh">Zhongwen</option>
                  <option value="ko">Hangugeo</option>
                </select>
              </label>
            </div>
          </section>

          {/* Appearance Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-green)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-green)]/30" />
              Apparence
            </h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Theme
                </span>
                <select
                  value={settings.theme}
                  onChange={(e) => updateSettings({ theme: e.target.value as 'light' | 'dark' | 'system' })}
                  className="select-field w-full"
                >
                  <option value="system">Systeme</option>
                  <option value="light">Clair</option>
                  <option value="dark">Sombre</option>
                </select>
              </label>
            </div>
          </section>

          {/* Options Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--text-secondary)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--border-subtle)]" />
              Options
            </h3>

            <div className="space-y-3">
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.auto_copy_to_clipboard}
                  onChange={(e) => updateSettings({ auto_copy_to_clipboard: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Copier automatiquement dans le presse-papier
                </span>
              </label>

              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.notification_on_complete}
                  onChange={(e) => updateSettings({ notification_on_complete: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Notification a la fin de la transcription
                </span>
              </label>

              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.minimize_to_tray}
                  onChange={(e) => updateSettings({ minimize_to_tray: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Minimiser dans la barre systeme
                </span>
              </label>
            </div>
          </section>

          {/* Shortcuts Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-cyan)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-cyan)]/30" />
              Raccourcis
            </h3>

            <div className="space-y-3">
              <div>
                <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Push-to-talk (maintenir)
                </label>
                <HotkeyInput
                  value={settings.hotkey_push_to_talk}
                  onChange={(hotkey) => updateSettings({ hotkey_push_to_talk: hotkey })}
                />
              </div>
              <div>
                <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Toggle enregistrement
                </label>
                <HotkeyInput
                  value={settings.hotkey_toggle_record}
                  onChange={(hotkey) => updateSettings({ hotkey_toggle_record: hotkey })}
                />
              </div>
            </div>
            <p className="text-[0.6rem] text-[var(--text-muted)]">
              Les raccourcis sont appliques immediatement.
            </p>
          </section>

          {/* Dictionary Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-magenta)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-magenta)]/30" />
              Dictionnaire
            </h3>

            <div className="flex gap-2">
              <input
                type="text"
                value={newWord}
                onChange={(e) => setNewWord(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleAddWord()}
                placeholder="Ajouter un mot..."
                className="input-field flex-1"
              />
              <button
                onClick={handleAddWord}
                className="btn-panel px-3 text-[var(--accent-cyan)] border-[var(--accent-cyan)]/30 hover:bg-[var(--accent-cyan)]/10"
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <line x1="12" y1="5" x2="12" y2="19" />
                  <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
              </button>
            </div>

            {dictionary.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {dictionary.map((word) => (
                  <span
                    key={word}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-sm text-[var(--text-secondary)] group"
                  >
                    {word}
                    <button
                      onClick={() => removeWord(word)}
                      className="opacity-40 hover:opacity-100 hover:text-[var(--accent-red)] transition-opacity"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                      </svg>
                    </button>
                  </span>
                ))}
              </div>
            )}
          </section>
        </div>

        {/* Footer */}
        <div className="flex-shrink-0 px-5 py-3 bg-[var(--bg-elevated)] border-t border-[var(--border-subtle)]">
          <p className="text-[0.6rem] text-[var(--text-muted)] text-center uppercase tracking-wider">
            WakaScribe v1.0.0 - Whisper.cpp
          </p>
        </div>
      </div>
    </div>
  );
}
