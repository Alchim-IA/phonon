import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
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
  const [apiKey, setApiKey] = useState('');
  const [showApiKey, setShowApiKey] = useState(false);
  const [apiKeyStatus, setApiKeyStatus] = useState<'valid' | 'invalid' | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
      loadDevices();
      loadDictionary();
      loadModels();
      checkApiKey();
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

  const checkApiKey = async () => {
    try {
      const hasKey = await invoke<boolean>('has_groq_api_key');
      if (hasKey) {
        setApiKey('••••••••••••••••');
        setApiKeyStatus('valid');
      }
    } catch (e) {
      console.error('Failed to check API key:', e);
    }
  };

  const handleSaveApiKey = async () => {
    if (!apiKey || apiKey === '••••••••••••••••') return;

    try {
      const isValid = await invoke<boolean>('validate_groq_api_key', { key: apiKey });
      if (isValid) {
        await invoke('set_groq_api_key', { key: apiKey });
        setApiKeyStatus('valid');
        setApiKey('••••••••••••••••');
      } else {
        setApiKeyStatus('invalid');
      }
    } catch (e) {
      console.error('Failed to save API key:', e);
      setApiKeyStatus('invalid');
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

  const handleDeleteModel = async (size: ModelSize) => {
    if (size === 'tiny') return; // Ne jamais supprimer tiny (bundled)
    if (settings?.whisper_model === size) {
      // Si le modèle actif est supprimé, basculer sur tiny
      await handleSwitchModel('tiny');
    }
    try {
      await invoke('delete_model', { size });
      await loadModels();
    } catch (e) {
      console.error('Delete failed:', e);
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
                    <div className="flex items-center gap-2">
                      {settings.whisper_model === model.size ? (
                        <span className="text-[0.6rem] text-[var(--accent-green)] uppercase">Actif</span>
                      ) : (
                        <button
                          onClick={() => handleSwitchModel(model.size)}
                          className="text-[0.6rem] text-[var(--accent-cyan)] hover:underline uppercase"
                        >
                          Utiliser
                        </button>
                      )}
                      {/* Bouton supprimer (sauf pour tiny qui est bundled) */}
                      {model.size !== 'tiny' && (
                        <button
                          onClick={() => handleDeleteModel(model.size)}
                          className="text-[0.6rem] text-[var(--text-muted)] hover:text-[var(--accent-red)] transition-colors"
                          title="Supprimer ce modèle"
                        >
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <polyline points="3 6 5 6 21 6" />
                            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                          </svg>
                        </button>
                      )}
                    </div>
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

          {/* LLM Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-cyan)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-cyan)]/30" />
              Intelligence (LLM)
            </h3>

            <div className="space-y-3">
              {/* Toggle LLM */}
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.llm_enabled}
                  onChange={(e) => updateSettings({ llm_enabled: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Activer le post-traitement LLM
                </span>
              </label>

              {/* Cle API Groq (visible seulement si LLM active) */}
              {settings.llm_enabled && (
                <>
                  <div>
                    <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                      Cle API Groq
                    </label>
                    <div className="flex gap-2">
                      <input
                        type={showApiKey ? 'text' : 'password'}
                        value={apiKey}
                        onChange={(e) => setApiKey(e.target.value)}
                        placeholder="gsk_..."
                        className="input-field flex-1"
                      />
                      <button
                        onClick={() => setShowApiKey(!showApiKey)}
                        className="btn-panel px-3"
                      >
                        {showApiKey ? '🙈' : '👁'}
                      </button>
                      <button
                        onClick={handleSaveApiKey}
                        className="btn-panel px-3 text-[var(--accent-green)]"
                      >
                        ✓
                      </button>
                    </div>
                    {apiKeyStatus && (
                      <p className={`text-[0.6rem] mt-1 ${apiKeyStatus === 'valid' ? 'text-[var(--accent-green)]' : 'text-[var(--accent-red)]'}`}>
                        {apiKeyStatus === 'valid' ? '✓ Cle valide' : '✗ Cle invalide'}
                      </p>
                    )}
                    <a
                      href="#"
                      onClick={(e) => { e.preventDefault(); openUrl('https://console.groq.com/keys'); }}
                      className="text-[0.6rem] text-[var(--accent-cyan)] hover:underline mt-1 inline-block"
                    >
                      Obtenir une cle gratuite →
                    </a>
                  </div>

                  {/* Mode LLM */}
                  <div>
                    <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                      Mode de correction
                    </label>
                    <div className="space-y-2">
                      {(['basic', 'smart', 'contextual'] as const).map((mode) => (
                        <label key={mode} className="flex items-center gap-2 cursor-pointer">
                          <input
                            type="radio"
                            name="llm_mode"
                            checked={settings.llm_mode === mode}
                            onChange={() => updateSettings({ llm_mode: mode })}
                            className="accent-[var(--accent-cyan)]"
                          />
                          <span className="text-sm text-[var(--text-secondary)]">
                            {mode === 'basic' && 'Basique - ponctuation et grammaire'}
                            {mode === 'smart' && 'Intelligent - reformulation claire'}
                            {mode === 'contextual' && 'Contextuel - adapte au mode de dictee'}
                          </span>
                        </label>
                      ))}
                    </div>
                  </div>
                </>
              )}
            </div>
          </section>

          {/* Translation Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-yellow)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-yellow)]/30" />
              Traduction
            </h3>

            <div className="space-y-3">
              {/* Toggle Traduction */}
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.translation_enabled}
                  onChange={(e) => updateSettings({ translation_enabled: e.target.checked })}
                />
                <span className="checkmark" />
                <div>
                  <span className="text-sm text-[var(--text-secondary)] block">
                    Traduction instantanee
                  </span>
                  <span className="text-[0.6rem] text-[var(--text-muted)]">
                    Traduit le texte du presse-papier via Groq
                  </span>
                </div>
              </label>

              {settings.translation_enabled && (
                <>
                  {/* Langue cible */}
                  <div>
                    <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                      Langue cible
                    </label>
                    <select
                      value={settings.translation_target_language}
                      onChange={(e) => updateSettings({ translation_target_language: e.target.value })}
                      className="select-field w-full"
                    >
                      <option value="en">English</option>
                      <option value="fr">Francais</option>
                      <option value="de">Deutsch</option>
                      <option value="es">Espanol</option>
                      <option value="it">Italiano</option>
                      <option value="pt">Portugues</option>
                      <option value="nl">Nederlands</option>
                      <option value="ru">Russkiy</option>
                      <option value="zh">Zhongwen</option>
                      <option value="ja">Nihongo</option>
                      <option value="ko">Hangugeo</option>
                      <option value="ar">Arabiy</option>
                    </select>
                  </div>

                  {/* Raccourci traduction */}
                  <div>
                    <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                      Raccourci traduction
                    </label>
                    <HotkeyInput
                      value={settings.hotkey_translate}
                      onChange={(hotkey) => updateSettings({ hotkey_translate: hotkey })}
                    />
                    <p className="text-[0.6rem] text-[var(--text-muted)] mt-1">
                      Copiez du texte, puis appuyez sur le raccourci pour traduire
                    </p>
                  </div>

                  {/* Avertissement API */}
                  {!apiKeyStatus && (
                    <div className="p-2 border border-[var(--accent-yellow)]/30 bg-[var(--accent-yellow)]/5 rounded">
                      <p className="text-[0.6rem] text-[var(--accent-yellow)]">
                        ⚠️ Une cle API Groq est requise pour la traduction.
                        Configurez-la dans la section Intelligence (LLM) ci-dessus.
                      </p>
                    </div>
                  )}
                </>
              )}
            </div>
          </section>

          {/* Dictation Mode Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-magenta)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-magenta)]/30" />
              Mode de dictee
            </h3>

            <div className="space-y-3">
              {/* Tabs mode dictee */}
              <div className="flex gap-1">
                {(['general', 'email', 'code', 'notes'] as const).map((mode) => (
                  <button
                    key={mode}
                    onClick={() => updateSettings({ dictation_mode: mode })}
                    className={`px-3 py-1.5 text-[0.65rem] uppercase tracking-wider border transition-colors ${
                      settings.dictation_mode === mode
                        ? 'border-[var(--accent-magenta)] text-[var(--accent-magenta)] bg-[var(--accent-magenta)]/10'
                        : 'border-[var(--border-subtle)] text-[var(--text-muted)] hover:border-[var(--accent-magenta)]/50'
                    }`}
                  >
                    {mode === 'general' && 'General'}
                    {mode === 'email' && 'Email'}
                    {mode === 'code' && 'Code'}
                    {mode === 'notes' && 'Notes'}
                  </button>
                ))}
              </div>

              {/* Toggle commandes vocales */}
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.voice_commands_enabled}
                  onChange={(e) => updateSettings({ voice_commands_enabled: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Commandes vocales activees
                </span>
              </label>
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

          {/* System Integration Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-yellow)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-yellow)]/30" />
              Integration Systeme
            </h3>

            <div className="space-y-3">
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.streaming_enabled}
                  onChange={(e) => updateSettings({ streaming_enabled: e.target.checked })}
                />
                <span className="checkmark" />
                <div>
                  <span className="text-sm text-[var(--text-secondary)] block">
                    Streaming temps reel
                  </span>
                  <span className="text-[0.6rem] text-[var(--text-muted)]">
                    Affiche le texte pendant l'enregistrement
                  </span>
                </div>
              </label>

              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.auto_paste_enabled}
                  onChange={(e) => updateSettings({ auto_paste_enabled: e.target.checked })}
                />
                <span className="checkmark" />
                <div>
                  <span className="text-sm text-[var(--text-secondary)] block">
                    Coller automatiquement
                  </span>
                  <span className="text-[0.6rem] text-[var(--text-muted)]">
                    Colle le texte dans l'app active apres transcription
                  </span>
                </div>
              </label>

              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.floating_window_enabled}
                  onChange={(e) => updateSettings({ floating_window_enabled: e.target.checked })}
                />
                <span className="checkmark" />
                <div>
                  <span className="text-sm text-[var(--text-secondary)] block">
                    Fenetre flottante
                  </span>
                  <span className="text-[0.6rem] text-[var(--text-muted)]">
                    Affiche une mini-fenetre toujours visible
                  </span>
                </div>
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
