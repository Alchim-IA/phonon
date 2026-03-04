import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  AppSettings,
  LocalLlmModel,
  DownloadProgress,
  LlmDownloadProgress,
  GroqQuota,
} from '../../types';

interface LlmSectionProps {
  settings: AppSettings;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
  onApiKeyStatusChange?: (status: 'valid' | 'invalid' | null) => void;
}

export function LlmSection({ settings, updateSettings, onApiKeyStatusChange }: LlmSectionProps) {
  const [apiKey, setApiKey] = useState('');
  const [showApiKey, setShowApiKey] = useState(false);
  const [apiKeyStatus, setApiKeyStatus] = useState<'valid' | 'invalid' | null>(null);
  const [groqQuota, setGroqQuota] = useState<GroqQuota | null>(null);
  const [llmModelsAvailable, setLlmModelsAvailable] = useState<LocalLlmModel[]>([]);
  const [downloadingLlm, setDownloadingLlm] = useState<LocalLlmModel | null>(null);
  const [llmDownloadProgress, setLlmDownloadProgress] = useState<DownloadProgress | null>(null);
  const [llmDownloadError, setLlmDownloadError] = useState<string | null>(null);

  // Propager le statut de la clé API au parent
  useEffect(() => {
    onApiKeyStatusChange?.(apiKeyStatus);
  }, [apiKeyStatus, onApiKeyStatusChange]);

  const loadLlmModels = async () => {
    try {
      const result = await invoke<LocalLlmModel[]>('get_available_llm_models');
      setLlmModelsAvailable(result);
    } catch (e) {
      console.error('Failed to load LLM models:', e);
    }
  };

  const loadGroqQuota = async () => {
    try {
      const quota = await invoke<GroqQuota | null>('get_groq_quota');
      setGroqQuota(quota);
    } catch (e) {
      console.error('Failed to load Groq quota:', e);
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

  useEffect(() => {
    loadLlmModels();
    checkApiKey();
    loadGroqQuota();
  }, []);

  useEffect(() => {
    const unlistenLlmProgress = listen<LlmDownloadProgress>('llm-download-progress', (event) => {
      setLlmDownloadProgress({
        downloaded: event.payload.downloaded,
        total: event.payload.total,
        percent: event.payload.progress
      });
    });

    return () => {
      unlistenLlmProgress.then(fn => fn());
    };
  }, []);

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

  const handleDownloadLlmModel = async (size: LocalLlmModel) => {
    setDownloadingLlm(size);
    setLlmDownloadError(null);
    setLlmDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    try {
      await invoke('download_llm_model', { modelSize: size });
      await loadLlmModels();
    } catch (e) {
      console.error('LLM download failed:', e);
      setLlmDownloadError(String(e));
    } finally {
      setDownloadingLlm(null);
      setLlmDownloadProgress(null);
    }
  };

  const handleDeleteLlmModel = async (size: LocalLlmModel) => {
    try {
      await invoke('delete_llm_model', { modelSize: size });
      await loadLlmModels();
    } catch (e) {
      console.error('Failed to delete LLM model:', e);
    }
  };

  return (
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
            {/* Provider Selection */}
            <div>
              <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block">Provider de resume</label>
              <div className="flex gap-2">
                <button
                  onClick={() => updateSettings({ llm_provider: 'groq' })}
                  className={`btn-glass flex-1 ${settings.llm_provider === 'groq' ? 'border-[var(--accent-primary)] bg-[var(--accent-primary-soft)]' : ''}`}
                >
                  <span className="w-2 h-2 rounded-full bg-blue-500" />
                  Cloud (Groq)
                </button>
                <button
                  onClick={() => updateSettings({ llm_provider: 'local' })}
                  className={`btn-glass flex-1 ${settings.llm_provider === 'local' ? 'border-[var(--accent-primary)] bg-[var(--accent-primary-soft)]' : ''}`}
                >
                  <span className="w-2 h-2 rounded-full bg-green-500" />
                  Local
                </button>
              </div>
            </div>

            {/* Groq Configuration */}
            {settings.llm_provider === 'groq' && (
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

                {/* Groq Quota Display */}
                {groqQuota && apiKeyStatus === 'valid' && (
                  <div className="mt-4 p-4 glass-card space-y-3">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-[0.8rem] text-[var(--text-primary)] font-medium">Quotas API</span>
                      <button
                        onClick={loadGroqQuota}
                        className="text-[var(--text-muted)] hover:text-[var(--accent-primary)] transition-colors"
                        title="Rafraichir"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                          <path d="M3 3v5h5" />
                          <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                          <path d="M16 21h5v-5" />
                        </svg>
                      </button>
                    </div>

                    {/* Requetes par jour (RPD) */}
                    <div>
                      <div className="flex justify-between text-[0.75rem] text-[var(--text-muted)] mb-1">
                        <span>Requetes / jour (RPD)</span>
                        <span className="tabular-nums">
                          {groqQuota.remaining_requests?.toLocaleString() ?? '?'} / {groqQuota.limit_requests?.toLocaleString() ?? '?'}
                        </span>
                      </div>
                      <div className="h-2 bg-[rgba(255,255,255,0.1)] rounded-full overflow-hidden">
                        <div
                          className="h-full bg-gradient-to-r from-[var(--accent-success)] to-[var(--accent-primary)] transition-all"
                          style={{ width: `${groqQuota.limit_requests ? ((groqQuota.remaining_requests ?? 0) / groqQuota.limit_requests * 100) : 0}%` }}
                        />
                      </div>
                      {groqQuota.reset_requests && (
                        <div className="text-[0.65rem] text-[var(--text-muted)] mt-1">
                          Reset dans: {groqQuota.reset_requests}
                        </div>
                      )}
                    </div>

                    {/* Tokens par minute (TPM) */}
                    <div>
                      <div className="flex justify-between text-[0.75rem] text-[var(--text-muted)] mb-1">
                        <span>Tokens / min (TPM)</span>
                        <span className="tabular-nums">
                          {groqQuota.remaining_tokens?.toLocaleString() ?? '?'} / {groqQuota.limit_tokens?.toLocaleString() ?? '?'}
                        </span>
                      </div>
                      <div className="h-2 bg-[rgba(255,255,255,0.1)] rounded-full overflow-hidden">
                        <div
                          className="h-full bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] transition-all"
                          style={{ width: `${groqQuota.limit_tokens ? ((groqQuota.remaining_tokens ?? 0) / groqQuota.limit_tokens * 100) : 0}%` }}
                        />
                      </div>
                      {groqQuota.reset_tokens && (
                        <div className="text-[0.65rem] text-[var(--text-muted)] mt-1">
                          Reset dans: {groqQuota.reset_tokens}
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Local LLM Configuration */}
            {settings.llm_provider === 'local' && (
              <div className="space-y-4">
                <div>
                  <label className="text-[0.8rem] text-[var(--text-muted)] mb-3 block">Modele LLM Local</label>
                  <div className="space-y-2">
                    {(['smollm2_360m', 'phi3_mini', 'qwen2_5_3b'] as LocalLlmModel[]).map((size) => {
                      const isAvailable = llmModelsAvailable.includes(size);
                      const isDownloading = downloadingLlm === size;
                      const isSelected = settings.local_llm_model === size;
                      const displayName = size === 'smollm2_360m'
                        ? 'SmolLM2 360M (386 MB) - Rapide'
                        : size === 'phi3_mini'
                        ? 'Phi-3 Mini (2.2 GB) - Recommande'
                        : 'Qwen2.5 3B (2 GB) - Qualite';

                      return (
                        <div
                          key={size}
                          className={`glass-card p-3 flex items-center justify-between ${
                            isSelected && isAvailable ? 'border-[var(--accent-primary)]' : ''
                          }`}
                        >
                          <div className="flex items-center gap-3">
                            {isAvailable && (
                              <input
                                type="radio"
                                name="local_llm_model"
                                checked={isSelected}
                                onChange={() => updateSettings({ local_llm_model: size })}
                                className="accent-[var(--accent-primary)]"
                              />
                            )}
                            <div>
                              <span className="text-[0.875rem] text-[var(--text-primary)]">{displayName}</span>
                              {isAvailable && (
                                <span className="tag-frost success text-[0.65rem] ml-2">Installe</span>
                              )}
                            </div>
                          </div>
                          <div className="flex items-center gap-2">
                            {isDownloading ? (
                              <div className="flex items-center gap-2">
                                <div className="w-24 h-1.5 bg-[rgba(255,255,255,0.1)] rounded-full overflow-hidden">
                                  <div
                                    className="h-full bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] transition-all"
                                    style={{ width: `${llmDownloadProgress?.percent || 0}%` }}
                                  />
                                </div>
                                <span className="text-[0.7rem] text-[var(--text-muted)]">
                                  {llmDownloadProgress?.percent || 0}%
                                </span>
                              </div>
                            ) : isAvailable ? (
                              <button
                                onClick={() => handleDeleteLlmModel(size)}
                                className="text-[var(--text-muted)] hover:text-[var(--accent-danger)] transition-colors"
                                title="Supprimer"
                              >
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                  <polyline points="3 6 5 6 21 6" />
                                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                                </svg>
                              </button>
                            ) : (
                              <button
                                onClick={() => handleDownloadLlmModel(size)}
                                className="btn-glass text-[0.75rem] py-1 px-2"
                              >
                                Telecharger
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>

                {llmDownloadError && (
                  <div className="glass-card p-4 border-[var(--accent-danger)]">
                    <p className="text-[0.8rem] text-[var(--accent-danger)]">
                      ❌ Erreur de telechargement: {llmDownloadError}
                    </p>
                  </div>
                )}

                {llmModelsAvailable.length === 0 && !llmDownloadError && (
                  <div className="glass-card p-4 border-[var(--accent-warning)]">
                    <p className="text-[0.8rem] text-[var(--accent-warning)]">
                      ⚠️ Aucun modele LLM local installe. Telechargez un modele ci-dessus.
                    </p>
                  </div>
                )}
              </div>
            )}

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
  );
}
