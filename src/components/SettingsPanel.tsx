import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useSettingsStore } from '../stores/settingsStore';
import { HotkeyInput } from './HotkeyInput';
import { ModelSize, ModelInfo, DownloadProgress, EngineType, VoskLanguage, VoskModelInfo, ParakeetModelSize, ParakeetModelInfo } from '../types';
import logoSvg from '../assets/logo.svg';

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
  const [voskModels, setVoskModels] = useState<VoskModelInfo[]>([]);
  const [downloadingVoskLang, setDownloadingVoskLang] = useState<VoskLanguage | null>(null);
  const [voskDownloadProgress, setVoskDownloadProgress] = useState<DownloadProgress | null>(null);
  const [parakeetModels, setParakeetModels] = useState<ParakeetModelInfo[]>([]);
  const [downloadingParakeet, setDownloadingParakeet] = useState<ParakeetModelSize | null>(null);
  const [parakeetDownloadProgress, setParakeetDownloadProgress] = useState<DownloadProgress | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
      loadDevices();
      loadDictionary();
      loadModels();
      loadVoskModels();
      loadParakeetModels();
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

    const unlistenVoskProgress = listen<DownloadProgress>('vosk-download-progress', (event) => {
      setVoskDownloadProgress(event.payload);
    });

    const unlistenVoskComplete = listen<VoskLanguage>('vosk-download-complete', () => {
      setDownloadingVoskLang(null);
      setVoskDownloadProgress(null);
      loadVoskModels();
    });

    const unlistenParakeetProgress = listen<DownloadProgress>('parakeet-download-progress', (event) => {
      setParakeetDownloadProgress(event.payload);
    });

    const unlistenParakeetComplete = listen<ParakeetModelSize>('parakeet-download-complete', () => {
      setDownloadingParakeet(null);
      setParakeetDownloadProgress(null);
      loadParakeetModels();
    });

    return () => {
      unlistenProgress.then(fn => fn());
      unlistenComplete.then(fn => fn());
      unlistenVoskProgress.then(fn => fn());
      unlistenVoskComplete.then(fn => fn());
      unlistenParakeetProgress.then(fn => fn());
      unlistenParakeetComplete.then(fn => fn());
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

  const loadVoskModels = async () => {
    try {
      const result = await invoke<VoskModelInfo[]>('get_vosk_models');
      setVoskModels(result);
    } catch (e) {
      console.error('Failed to load Vosk models:', e);
    }
  };

  const loadParakeetModels = async () => {
    try {
      const result = await invoke<ParakeetModelInfo[]>('get_parakeet_models');
      setParakeetModels(result);
    } catch (e) {
      console.error('Failed to load Parakeet models:', e);
    }
  };

  const handleDownloadParakeetModel = async (size: ParakeetModelSize) => {
    setDownloadingParakeet(size);
    setParakeetDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    try {
      await invoke('download_parakeet_model', { size });
    } catch (e) {
      console.error('Parakeet download failed:', e);
      setDownloadingParakeet(null);
      setParakeetDownloadProgress(null);
    }
  };

  const handleDeleteParakeetModel = async (size: ParakeetModelSize) => {
    try {
      await invoke('delete_parakeet_model', { size });
      await loadParakeetModels();
    } catch (e) {
      console.error('Failed to delete Parakeet model:', e);
    }
  };

  const handleSelectParakeetModel = async (size: ParakeetModelSize) => {
    try {
      await invoke('select_parakeet_model', { size });
      await loadSettings();
    } catch (e) {
      console.error('Failed to select Parakeet model:', e);
    }
  };

  const handleSwitchEngine = async (engineType: EngineType) => {
    try {
      await updateSettings({ engine_type: engineType });
    } catch (e) {
      console.error('Failed to switch engine:', e);
    }
  };

  const handleDownloadVoskModel = async (language: VoskLanguage) => {
    setDownloadingVoskLang(language);
    setVoskDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    try {
      await invoke('download_vosk_model', { language });
    } catch (e) {
      console.error('Vosk download failed:', e);
      setDownloadingVoskLang(null);
      setVoskDownloadProgress(null);
    }
  };

  const handleSelectVoskLanguage = async (language: VoskLanguage) => {
    try {
      await invoke('select_vosk_language', { language });
      await loadSettings();
    } catch (e) {
      console.error('Failed to select Vosk language:', e);
    }
  };

  const checkApiKey = async () => {
    try {
      const hasKey = await invoke<boolean>('has_groq_api_key');
      if (hasKey) {
        const key = await invoke<string | null>('get_groq_api_key');
        if (key) {
          setApiKey(key);
          setShowApiKey(false);
        } else {
          setApiKey('••••••••••••••••');
        }
        setApiKeyStatus('valid');
      }
    } catch (e) {
      console.error('Failed to check API key:', e);
    }
  };

  const handleSaveApiKey = async () => {
    if (!apiKey || apiKey === '••••••••••••••••') return;

    try {
      await invoke('set_groq_api_key', { key: apiKey });

      try {
        const isValid = await invoke<boolean>('validate_groq_api_key', { key: apiKey });
        if (isValid) {
          setApiKeyStatus('valid');
        } else {
          setApiKeyStatus('invalid');
        }
      } catch {
        setApiKeyStatus('valid');
      }

      setShowApiKey(false);
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
    if (size === 'tiny') return;
    if (settings?.whisper_model === size) {
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
        className="settings-backdrop animate-fade-in"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="settings-panel-frost relative w-full max-w-md h-full bg-[#14142a] border-l border-[rgba(255,255,255,0.1)] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex-shrink-0 px-6 py-5 bg-[rgba(255,255,255,0.08)] border-b border-[rgba(255,255,255,0.1)] flex justify-between items-center">
          <div className="flex items-center gap-4">
            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)] flex items-center justify-center shadow-lg p-2">
              <img src={logoSvg} alt="WakaScribe" className="w-full h-full invert" />
            </div>
            <div>
              <h2 className="font-display text-lg text-[var(--text-primary)]">
                Parametres
              </h2>
              <p className="text-[0.75rem] text-[var(--text-muted)]">Configuration de WakaScribe</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="w-9 h-9 flex items-center justify-center rounded-xl bg-[rgba(255,255,255,0.08)] border border-[var(--glass-border)] hover:border-[var(--accent-danger)] hover:text-[var(--accent-danger)] transition-all"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-8 scrollbar-thin">
          {/* Audio Section */}
          <section className="space-y-4">
            <h3 className="section-title primary">Audio</h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.8rem] text-[rgba(255,255,255,0.75)] mb-2 block">Microphone</span>
                <select
                  value={settings.microphone_id || ''}
                  onChange={(e) => updateSettings({ microphone_id: e.target.value || null })}
                  className="select-glass"
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
            <h3 className="section-title success">Moteur de transcription</h3>

            {/* Engine Type Selector */}
            <div className="flex gap-2">
              {(['whisper', 'vosk', 'parakeet'] as EngineType[]).map((engine) => (
                <button
                  key={engine}
                  onClick={() => handleSwitchEngine(engine)}
                  className={`flex-1 px-4 py-2.5 text-[0.8rem] font-medium rounded-xl border transition-all ${
                    settings.engine_type === engine
                      ? 'bg-[var(--accent-success-soft)] border-[var(--accent-success)] text-[var(--accent-success)]'
                      : 'bg-[rgba(255,255,255,0.08)] border-[var(--glass-border)] text-[var(--text-muted)] hover:border-[var(--accent-success)]'
                  }`}
                >
                  {engine === 'whisper' && 'Whisper'}
                  {engine === 'vosk' && 'Vosk'}
                  {engine === 'parakeet' && 'Parakeet'}
                </button>
              ))}
            </div>

            {/* Whisper Models */}
            {settings.engine_type === 'whisper' && (
              <div className="space-y-3">
                <p className="text-[0.75rem] text-[var(--text-muted)]">
                  Whisper (OpenAI) - Haute precision, 99 langues
                </p>
                {models.map((model) => (
                  <div
                    key={model.size}
                    className={`glass-card p-4 ${
                      settings.whisper_model === model.size ? 'border-[var(--accent-success)]' : ''
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className={`w-3 h-3 rounded-full ${
                          settings.whisper_model === model.size
                            ? 'bg-[var(--accent-success)]'
                            : 'bg-[var(--glass-border)]'
                        }`} />
                        <div>
                          <div className="text-[0.9375rem] text-[var(--text-primary)] font-medium">
                            {model.display_name}
                          </div>
                          {model.size === 'small' && (
                            <div className="text-[0.7rem] text-[var(--accent-primary)]">Recommande</div>
                          )}
                        </div>
                      </div>

                      {downloading === model.size ? (
                        <div className="flex items-center gap-3">
                          <div className="w-24 progress-frost">
                            <div className="bar" style={{ width: `${downloadProgress?.percent || 0}%` }} />
                          </div>
                          <span className="text-[0.75rem] text-[var(--text-muted)] w-12 text-right tabular-nums">
                            {Math.round(downloadProgress?.percent || 0)}%
                          </span>
                        </div>
                      ) : model.available ? (
                        <div className="flex items-center gap-3">
                          {settings.whisper_model === model.size ? (
                            <span className="tag-frost success">Actif</span>
                          ) : (
                            <button
                              onClick={() => handleSwitchModel(model.size)}
                              className="text-[0.8rem] text-[var(--accent-primary)] hover:underline font-medium"
                            >
                              Utiliser
                            </button>
                          )}
                          {model.size !== 'tiny' && (
                            <button
                              onClick={() => handleDeleteModel(model.size)}
                              className="text-[var(--text-muted)] hover:text-[var(--accent-danger)] transition-colors p-1"
                            >
                              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                <polyline points="3 6 5 6 21 6" />
                                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                              </svg>
                            </button>
                          )}
                        </div>
                      ) : (
                        <button
                          onClick={() => handleDownloadModel(model.size)}
                          className="btn-glass text-[0.8rem]"
                        >
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                            <polyline points="7 10 12 15 17 10" />
                            <line x1="12" y1="15" x2="12" y2="3" />
                          </svg>
                          Telecharger
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Vosk Models */}
            {settings.engine_type === 'vosk' && (
              <div className="space-y-3">
                <p className="text-[0.75rem] text-[var(--text-muted)]">
                  Vosk - Leger et rapide, modeles par langue
                </p>
                <div className="grid grid-cols-2 gap-2">
                  {voskModels.map((model) => (
                    <div
                      key={model.language}
                      className={`glass-card p-3 ${
                        settings.vosk_language === model.language ? 'border-[var(--accent-success)]' : ''
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <div className={`w-2 h-2 rounded-full ${
                            settings.vosk_language === model.language
                              ? 'bg-[var(--accent-success)]'
                              : 'bg-[var(--glass-border)]'
                          }`} />
                          <span className="text-[0.8rem] text-[var(--text-primary)]">
                            {model.display_name}
                          </span>
                        </div>

                        {downloadingVoskLang === model.language ? (
                          <div className="flex items-center gap-1">
                            <div className="w-12 progress-frost">
                              <div className="bar" style={{ width: `${voskDownloadProgress?.percent || 0}%` }} />
                            </div>
                          </div>
                        ) : model.available ? (
                          settings.vosk_language === model.language ? (
                            <span className="text-[0.65rem] text-[var(--accent-success)]">Actif</span>
                          ) : (
                            <button
                              onClick={() => handleSelectVoskLanguage(model.language)}
                              className="text-[0.7rem] text-[var(--accent-primary)] hover:underline"
                            >
                              Utiliser
                            </button>
                          )
                        ) : (
                          <button
                            onClick={() => handleDownloadVoskModel(model.language)}
                            className="text-[var(--text-muted)] hover:text-[var(--accent-primary)] transition-colors"
                          >
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                              <polyline points="7 10 12 15 17 10" />
                              <line x1="12" y1="15" x2="12" y2="3" />
                            </svg>
                          </button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Parakeet Models */}
            {settings.engine_type === 'parakeet' && (
              <div className="space-y-3">
                <p className="text-[0.75rem] text-[var(--text-muted)]">
                  Parakeet TDT (NVIDIA) - Detection automatique, 25 langues europeennes
                </p>
                {parakeetModels.map((model) => (
                  <div
                    key={model.size}
                    className={`glass-card p-4 ${
                      settings.parakeet_model === model.size && model.available ? 'border-[var(--accent-success)]' : ''
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className={`w-3 h-3 rounded-full ${
                          settings.parakeet_model === model.size && model.available
                            ? 'bg-[var(--accent-success)]'
                            : 'bg-[var(--glass-border)]'
                        }`} />
                        <div>
                          <div className="text-[0.9375rem] text-[var(--text-primary)] font-medium">
                            {model.display_name}
                          </div>
                          <div className="text-[0.7rem] text-[var(--text-muted)]">
                            ~{(model.size_bytes / 1_000_000_000).toFixed(1)} GB
                          </div>
                        </div>
                      </div>

                      {downloadingParakeet === model.size ? (
                        <div className="flex items-center gap-3">
                          <div className="w-24 progress-frost">
                            <div className="bar" style={{ width: `${parakeetDownloadProgress?.percent || 0}%` }} />
                          </div>
                          <span className="text-[0.75rem] text-[var(--text-muted)] w-12 text-right tabular-nums">
                            {Math.round(parakeetDownloadProgress?.percent || 0)}%
                          </span>
                        </div>
                      ) : model.available ? (
                        <div className="flex items-center gap-3">
                          {settings.parakeet_model === model.size ? (
                            <span className="tag-frost success">Actif</span>
                          ) : (
                            <button
                              onClick={() => handleSelectParakeetModel(model.size)}
                              className="text-[0.8rem] text-[var(--accent-primary)] hover:underline font-medium"
                            >
                              Utiliser
                            </button>
                          )}
                          <button
                            onClick={() => handleDeleteParakeetModel(model.size)}
                            className="text-[var(--text-muted)] hover:text-[var(--accent-danger)] transition-colors p-1"
                          >
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <polyline points="3 6 5 6 21 6" />
                              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                            </svg>
                          </button>
                        </div>
                      ) : (
                        <button
                          onClick={() => handleDownloadParakeetModel(model.size)}
                          className="btn-glass text-[0.8rem]"
                        >
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                            <polyline points="7 10 12 15 17 10" />
                            <line x1="12" y1="15" x2="12" y2="3" />
                          </svg>
                          Telecharger
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>

          {/* LLM Section */}
          <section className="space-y-4">
            <h3 className="section-title primary">Intelligence (LLM)</h3>

            <div className="space-y-4">
              <label className="checkbox-frost">
                <input
                  type="checkbox"
                  checked={settings.llm_enabled}
                  onChange={(e) => updateSettings({ llm_enabled: e.target.checked })}
                />
                <span className="check-box" />
                <span className="check-label">Activer le post-traitement LLM</span>
              </label>

              {settings.llm_enabled && (
                <>
                  <div>
                    <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Cle API Groq</label>
                    <div className="flex gap-2">
                      <input
                        type={showApiKey ? 'text' : 'password'}
                        value={apiKey}
                        onChange={(e) => setApiKey(e.target.value)}
                        placeholder="gsk_..."
                        className="input-glass flex-1"
                      />
                      <button onClick={() => setShowApiKey(!showApiKey)} className="btn-glass px-3">
                        {showApiKey ? '🙈' : '👁'}
                      </button>
                      <button onClick={handleSaveApiKey} className="btn-glass px-3 text-[var(--accent-success)]">
                        ✓
                      </button>
                    </div>
                    {apiKeyStatus && (
                      <p className={`text-[0.75rem] mt-2 ${apiKeyStatus === 'valid' ? 'text-[var(--accent-success)]' : 'text-[var(--accent-danger)]'}`}>
                        {apiKeyStatus === 'valid' ? '✓ Cle valide' : '✗ Cle invalide'}
                      </p>
                    )}
                    <a
                      href="#"
                      onClick={(e) => { e.preventDefault(); openUrl('https://console.groq.com/keys'); }}
                      className="text-[0.75rem] text-[var(--accent-primary)] hover:underline mt-2 inline-block"
                    >
                      Obtenir une cle gratuite →
                    </a>
                  </div>

                  <div>
                    <label className="text-[0.8rem] text-[var(--text-muted)] mb-3 block">Mode de correction</label>
                    <div className="space-y-2">
                      {(['basic', 'smart', 'contextual'] as const).map((mode) => (
                        <label key={mode} className="radio-frost">
                          <input
                            type="radio"
                            name="llm_mode"
                            checked={settings.llm_mode === mode}
                            onChange={() => updateSettings({ llm_mode: mode })}
                          />
                          <span className="text-[0.9375rem] text-[var(--text-secondary)]">
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
            <h3 className="section-title warning">Traduction</h3>

            <div className="space-y-4">
              <label className="checkbox-frost">
                <input
                  type="checkbox"
                  checked={settings.translation_enabled}
                  onChange={(e) => updateSettings({ translation_enabled: e.target.checked })}
                />
                <span className="check-box" />
                <div>
                  <span className="check-label block">Traduction instantanee</span>
                  <span className="text-[0.75rem] text-[var(--text-muted)]">
                    Traduit le texte du presse-papier via Groq
                  </span>
                </div>
              </label>

              {settings.translation_enabled && (
                <>
                  <div>
                    <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Langue cible</label>
                    <select
                      value={settings.translation_target_language}
                      onChange={(e) => updateSettings({ translation_target_language: e.target.value })}
                      className="select-glass"
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

                  <div>
                    <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Raccourci traduction</label>
                    <HotkeyInput
                      value={settings.hotkey_translate}
                      onChange={(hotkey) => updateSettings({ hotkey_translate: hotkey })}
                    />
                    <p className="text-[0.75rem] text-[var(--text-muted)] mt-2">
                      Copiez du texte, puis appuyez sur le raccourci pour traduire
                    </p>
                  </div>

                  {!apiKeyStatus && (
                    <div className="glass-card p-4 border-[var(--accent-warning)]">
                      <p className="text-[0.8rem] text-[var(--accent-warning)]">
                        ⚠️ Une cle API Groq est requise pour la traduction.
                      </p>
                    </div>
                  )}
                </>
              )}
            </div>
          </section>

          {/* Dictation Mode Section */}
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
            </div>
          </section>

          {/* Transcription Section */}
          <section className="space-y-4">
            <h3 className="section-title secondary">Transcription</h3>

            <div>
              <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Langue</label>
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
                className="select-glass"
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
            </div>
          </section>

          {/* Options Section */}
          <section className="space-y-4">
            <h3 className="section-title">Options</h3>

            <div className="space-y-3">
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

          {/* System Integration Section */}
          <section className="space-y-4">
            <h3 className="section-title warning">Integration Systeme</h3>

            <div className="space-y-3">
              <label className="checkbox-frost">
                <input
                  type="checkbox"
                  checked={settings.streaming_enabled}
                  onChange={(e) => updateSettings({ streaming_enabled: e.target.checked })}
                />
                <span className="check-box" />
                <div>
                  <span className="check-label block">Streaming temps reel</span>
                  <span className="text-[0.75rem] text-[var(--text-muted)]">Affiche le texte pendant l'enregistrement</span>
                </div>
              </label>

              <label className="checkbox-frost">
                <input
                  type="checkbox"
                  checked={settings.auto_paste_enabled}
                  onChange={(e) => updateSettings({ auto_paste_enabled: e.target.checked })}
                />
                <span className="check-box" />
                <div>
                  <span className="check-label block">Coller automatiquement</span>
                  <span className="text-[0.75rem] text-[var(--text-muted)]">Colle le texte dans l'app active apres transcription</span>
                </div>
              </label>

              <label className="checkbox-frost">
                <input
                  type="checkbox"
                  checked={settings.floating_window_enabled}
                  onChange={async (e) => {
                    const enabled = e.target.checked;
                    await updateSettings({ floating_window_enabled: enabled });
                    try {
                      if (enabled) {
                        await invoke('show_floating_window');
                      } else {
                        await invoke('hide_floating_window');
                      }
                    } catch (err) {
                      console.error('Failed to toggle floating window:', err);
                    }
                  }}
                />
                <span className="check-box" />
                <div>
                  <span className="check-label block">Fenetre flottante</span>
                  <span className="text-[0.75rem] text-[var(--text-muted)]">Affiche une mini-fenetre toujours visible</span>
                </div>
              </label>
            </div>
          </section>

          {/* Shortcuts Section */}
          <section className="space-y-4">
            <h3 className="section-title primary">Raccourcis</h3>

            <div className="space-y-4">
              <div>
                <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Push-to-talk (maintenir)</label>
                <HotkeyInput
                  value={settings.hotkey_push_to_talk}
                  onChange={(hotkey) => updateSettings({ hotkey_push_to_talk: hotkey })}
                />
              </div>
              <div>
                <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Toggle enregistrement</label>
                <HotkeyInput
                  value={settings.hotkey_toggle_record}
                  onChange={(hotkey) => updateSettings({ hotkey_toggle_record: hotkey })}
                />
              </div>
            </div>
            <p className="text-[0.75rem] text-[var(--text-muted)]">
              Les raccourcis sont appliques immediatement.
            </p>
          </section>

          {/* Dictionary Section */}
          <section className="space-y-4">
            <h3 className="section-title secondary">Dictionnaire</h3>

            <div className="flex gap-2">
              <input
                type="text"
                value={newWord}
                onChange={(e) => setNewWord(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleAddWord()}
                placeholder="Ajouter un mot..."
                className="input-glass flex-1"
              />
              <button
                onClick={handleAddWord}
                className="btn-glass px-4 text-[var(--accent-primary)]"
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
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
                    className="tag-frost group"
                  >
                    {word}
                    <button
                      onClick={() => removeWord(word)}
                      className="opacity-50 hover:opacity-100 hover:text-[var(--accent-danger)] transition-opacity ml-1"
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
        <div className="flex-shrink-0 px-6 py-4 bg-[rgba(255,255,255,0.08)] border-t border-[rgba(255,255,255,0.1)]">
          <p className="text-[0.75rem] text-[var(--text-muted)] text-center">
            WakaScribe v1.0.0 - {settings.engine_type === 'whisper' ? 'Whisper.cpp' : settings.engine_type === 'vosk' ? 'Vosk' : 'Parakeet'}
          </p>
        </div>
      </div>
    </div>
  );
}
