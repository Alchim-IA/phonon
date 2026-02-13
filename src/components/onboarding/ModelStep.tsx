import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  ModelInfo,
  ModelSize,
  DownloadProgress,
  EngineType,
  VoskModelInfo,
  VoskLanguage,
  ParakeetModelInfo,
  ParakeetModelSize,
} from '../../types';
import { useSettingsStore } from '../../stores/settingsStore';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

const ENGINE_INFO: Record<EngineType, { description: string }> = {
  whisper: { description: 'OpenAI — Haute precision, 99 langues' },
  vosk: { description: 'Leger et rapide, modeles par langue' },
  parakeet: { description: 'NVIDIA — Detection auto, 25 langues (macOS)' },
};

export function ModelStep({ onValidChange }: StepProps) {
  const { settings, updateSettings } = useSettingsStore();
  const [selectedEngine, setSelectedEngine] = useState<EngineType>(settings?.engine_type || 'whisper');

  // Whisper state
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<ModelSize | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadComplete, setDownloadComplete] = useState(false);

  // Vosk state
  const [voskModels, setVoskModels] = useState<VoskModelInfo[]>([]);
  const [downloadingVoskLang, setDownloadingVoskLang] = useState<VoskLanguage | null>(null);
  const [voskDownloadProgress, setVoskDownloadProgress] = useState<DownloadProgress | null>(null);

  // Parakeet state
  const [parakeetModels, setParakeetModels] = useState<ParakeetModelInfo[]>([]);
  const [downloadingParakeet, setDownloadingParakeet] = useState<ParakeetModelSize | null>(null);
  const [parakeetDownloadProgress, setParakeetDownloadProgress] = useState<DownloadProgress | null>(null);

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

  useEffect(() => {
    loadModels();
    loadVoskModels();
    loadParakeetModels();
  }, []);

  // Block navigation while any download is in progress
  useEffect(() => {
    const anyDownloading = downloading !== null || downloadingVoskLang !== null || downloadingParakeet !== null;
    onValidChange(!anyDownloading);
  }, [downloading, downloadingVoskLang, downloadingParakeet, onValidChange]);

  useEffect(() => {
    // Whisper events
    const unlistenProgress = listen<DownloadProgress>('model-download-progress', (event) => {
      setDownloadProgress(event.payload);
    });
    const unlistenComplete = listen<ModelSize>('model-download-complete', () => {
      setDownloading(null);
      setDownloadProgress(null);
      setDownloadComplete(true);
      loadModels();
    });

    // Vosk events
    const unlistenVoskProgress = listen<DownloadProgress>('vosk-download-progress', (event) => {
      setVoskDownloadProgress(event.payload);
    });
    const unlistenVoskComplete = listen<VoskLanguage>('vosk-download-complete', () => {
      setDownloadingVoskLang(null);
      setVoskDownloadProgress(null);
      setDownloadComplete(true);
      loadVoskModels();
    });

    // Parakeet events
    const unlistenParakeetProgress = listen<DownloadProgress>('parakeet-download-progress', (event) => {
      setParakeetDownloadProgress(event.payload);
    });
    const unlistenParakeetComplete = listen<ParakeetModelSize>('parakeet-download-complete', () => {
      setDownloadingParakeet(null);
      setParakeetDownloadProgress(null);
      setDownloadComplete(true);
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

  const handleEngineChange = async (engine: EngineType) => {
    setSelectedEngine(engine);
    setDownloadComplete(false);
    await updateSettings({ engine_type: engine });
  };

  const handleDownload = async (size: ModelSize) => {
    setDownloading(size);
    setDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    setDownloadComplete(false);
    try {
      await invoke('download_model', { size });
    } catch (e) {
      console.error('Download failed:', e);
      setDownloading(null);
      setDownloadProgress(null);
    }
  };

  const handleDownloadVosk = async (language: VoskLanguage) => {
    setDownloadingVoskLang(language);
    setVoskDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    setDownloadComplete(false);
    try {
      await invoke('download_vosk_model', { language });
    } catch (e) {
      console.error('Vosk download failed:', e);
      setDownloadingVoskLang(null);
      setVoskDownloadProgress(null);
    }
  };

  const handleDownloadParakeet = async (size: ParakeetModelSize) => {
    setDownloadingParakeet(size);
    setParakeetDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    setDownloadComplete(false);
    try {
      await invoke('download_parakeet_model', { size });
    } catch (e) {
      console.error('Parakeet download failed:', e);
      setDownloadingParakeet(null);
      setParakeetDownloadProgress(null);
    }
  };

  const qualityLabels: Record<ModelSize, string> = {
    tiny: 'Basique',
    small: 'Bonne',
    medium: 'Tres bonne',
  };

  const qualityColors: Record<ModelSize, string> = {
    tiny: 'var(--text-muted)',
    small: 'var(--accent-primary)',
    medium: 'var(--accent-success)',
  };

  const downloadIcon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  );

  return (
    <div className="py-4">
      <div className="text-center mb-5">
        <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
          Moteur de reconnaissance vocale
        </h2>
        <p className="text-[var(--text-secondary)] text-[0.85rem]">
          Choisissez le moteur et telechargez les modeles souhaites.
        </p>
      </div>

      {/* Engine selector */}
      <div className="flex gap-2 mb-5">
        {(['whisper', 'vosk', 'parakeet'] as EngineType[]).map((engine) => (
          <button
            key={engine}
            onClick={() => handleEngineChange(engine)}
            className={`flex-1 px-4 py-2.5 text-[0.8rem] font-medium rounded-xl border transition-all ${
              selectedEngine === engine
                ? 'bg-[var(--accent-success-soft,rgba(122,239,178,0.1))] border-[var(--accent-success)] text-[var(--accent-success)]'
                : 'bg-[rgba(255,255,255,0.08)] border-[var(--glass-border)] text-[var(--text-muted)] hover:border-[var(--accent-success)]'
            }`}
          >
            {engine === 'whisper' && 'Whisper'}
            {engine === 'vosk' && 'Vosk'}
            {engine === 'parakeet' && 'Parakeet'}
          </button>
        ))}
      </div>

      <p className="text-[0.75rem] text-[var(--text-muted)] mb-4">
        {ENGINE_INFO[selectedEngine].description}
      </p>

      {/* Whisper models */}
      {selectedEngine === 'whisper' && (
        <div className="space-y-3">
          {models.map((model) => (
            <div
              key={model.size}
              className={`glass-card p-4 transition-all ${
                model.available && model.size !== 'tiny' ? 'border-[var(--accent-success)]' : ''
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className={`w-9 h-9 rounded-xl flex items-center justify-center ${
                    model.size === 'tiny'
                      ? 'bg-[rgba(255,255,255,0.08)]'
                      : model.size === 'small'
                      ? 'bg-[rgba(124,138,255,0.15)]'
                      : 'bg-[rgba(122,239,178,0.15)]'
                  }`}>
                    <span className="text-[0.8rem] font-medium" style={{ color: qualityColors[model.size] }}>
                      {model.size === 'tiny' ? 'T' : model.size === 'small' ? 'S' : 'M'}
                    </span>
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-[0.9rem] text-[var(--text-primary)] font-medium">
                        {model.display_name}
                      </span>
                      {model.size === 'tiny' && (
                        <span className="tag-frost text-[0.6rem]">Inclus</span>
                      )}
                      {model.size === 'small' && (
                        <span className="text-[0.65rem] text-[var(--accent-primary)]">Recommande</span>
                      )}
                    </div>
                    <span className="text-[0.7rem]" style={{ color: qualityColors[model.size] }}>
                      Qualite: {qualityLabels[model.size]}
                    </span>
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
                  <span className="tag-frost success">Installe</span>
                ) : (
                  <button
                    onClick={() => handleDownload(model.size)}
                    disabled={downloading !== null}
                    className="btn-glass text-[0.8rem]"
                  >
                    {downloadIcon}
                    Telecharger
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Vosk models */}
      {selectedEngine === 'vosk' && (
        <div className="grid grid-cols-3 gap-2">
          {voskModels.map((model) => (
            <div
              key={model.language}
              className={`glass-card p-3 transition-all ${
                model.available ? 'border-[var(--accent-success)]' : ''
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="text-[0.8rem] text-[var(--text-primary)]">
                  {model.display_name}
                </span>

                {downloadingVoskLang === model.language ? (
                  <div className="flex items-center gap-1">
                    <div className="w-12 progress-frost">
                      <div className="bar" style={{ width: `${voskDownloadProgress?.percent || 0}%` }} />
                    </div>
                  </div>
                ) : model.available ? (
                  <span className="text-[0.65rem] text-[var(--accent-success)]">Installe</span>
                ) : (
                  <button
                    onClick={() => handleDownloadVosk(model.language)}
                    disabled={downloadingVoskLang !== null}
                    className="text-[var(--text-muted)] hover:text-[var(--accent-primary)] transition-colors"
                  >
                    {downloadIcon}
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Parakeet models */}
      {selectedEngine === 'parakeet' && (
        <div className="space-y-3">
          {parakeetModels.map((model) => (
            <div
              key={model.size}
              className={`glass-card p-4 transition-all ${
                model.available ? 'border-[var(--accent-success)]' : ''
              }`}
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[0.9rem] text-[var(--text-primary)] font-medium">
                    {model.display_name}
                  </div>
                  <div className="text-[0.7rem] text-[var(--text-muted)]">
                    ~{(model.size_bytes / 1_000_000_000).toFixed(1)} GB
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
                  <span className="tag-frost success">Installe</span>
                ) : (
                  <button
                    onClick={() => handleDownloadParakeet(model.size)}
                    disabled={downloadingParakeet !== null}
                    className="btn-glass text-[0.8rem]"
                  >
                    {downloadIcon}
                    Telecharger
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {downloadComplete && (
        <p className="text-center text-[var(--accent-success)] text-[0.8rem] mt-4">
          Modele telecharge avec succes !
        </p>
      )}
    </div>
  );
}
