import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AudioDevice } from '../../types';
import { useSettingsStore } from '../../stores/settingsStore';
import { AudioWaveform } from '../AudioWaveform';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

type PermissionStatus = 'checking' | 'granted' | 'denied';

export function PermissionStep({ onValidChange }: StepProps) {
  const [status, setStatus] = useState<PermissionStatus>('checking');
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [previewActive, setPreviewActive] = useState(false);
  const { updateSettings } = useSettingsStore();

  const startPreview = useCallback(async (deviceId: string | null) => {
    try {
      await invoke('start_mic_preview', { deviceId });
      setPreviewActive(true);
    } catch (e) {
      console.error('Failed to start mic preview:', e);
    }
  }, []);

  const stopPreview = useCallback(async () => {
    try {
      await invoke('stop_mic_preview');
      setPreviewActive(false);
    } catch (e) {
      console.error('Failed to stop mic preview:', e);
    }
  }, []);

  useEffect(() => {
    checkMicrophoneAccess();
    return () => {
      invoke('stop_mic_preview').catch(() => {});
    };
  }, []);

  useEffect(() => {
    onValidChange(status === 'granted');
  }, [status, onValidChange]);

  const checkMicrophoneAccess = async () => {
    try {
      const deviceList = await invoke<AudioDevice[]>('list_audio_devices');
      if (deviceList.length > 0) {
        setDevices(deviceList);
        setStatus('granted');
        const defaultDevice = deviceList.find(d => d.is_default) || deviceList[0];
        setSelectedDevice(defaultDevice.id);
        await updateSettings({ microphone_id: defaultDevice.id });
        startPreview(defaultDevice.id);
      } else {
        setStatus('denied');
      }
    } catch {
      setStatus('denied');
    }
  };

  const handleDeviceChange = async (deviceId: string) => {
    setSelectedDevice(deviceId);
    await updateSettings({ microphone_id: deviceId });
    await stopPreview();
    startPreview(deviceId);
  };

  return (
    <div className="flex flex-col items-center text-center py-6">
      <div className={`w-20 h-20 rounded-full flex items-center justify-center mb-6 transition-all ${
        status === 'checking'
          ? 'bg-[rgba(255,255,255,0.08)]'
          : status === 'granted'
          ? 'bg-[rgba(122,239,178,0.15)]'
          : 'bg-[rgba(255,122,122,0.15)]'
      }`}>
        {status === 'checking' ? (
          <div className="w-8 h-8 border-2 border-[var(--text-muted)] border-t-[var(--accent-primary)] rounded-full animate-spin" />
        ) : status === 'granted' ? (
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="var(--accent-success)" strokeWidth="1.5">
            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <line x1="12" x2="12" y1="19" y2="22" />
          </svg>
        ) : (
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="var(--accent-danger)" strokeWidth="1.5">
            <line x1="2" y1="2" x2="22" y2="22" />
            <path d="M18.89 13.23A7.12 7.12 0 0 0 19 12v-2" />
            <path d="M5 10v2a7 7 0 0 0 12 5" />
            <path d="M15 9.34V5a3 3 0 0 0-5.68-1.33" />
            <path d="M9 9v3a3 3 0 0 0 5.12 2.12" />
            <line x1="12" x2="12" y1="19" y2="22" />
          </svg>
        )}
      </div>

      <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
        Acces au microphone
      </h2>

      {status === 'checking' && (
        <p className="text-[var(--text-secondary)] text-[0.85rem] mb-4">
          Verification de l'acces au microphone...
        </p>
      )}

      {status === 'granted' && (
        <>
          <p className="text-[var(--accent-success)] text-[0.85rem] mb-6">
            Microphone accessible
          </p>

          <div className="w-full max-w-sm mb-6">
            <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block text-left">
              Microphone
            </label>
            <select
              value={selectedDevice || ''}
              onChange={(e) => handleDeviceChange(e.target.value)}
              className="w-full px-4 py-2.5 rounded-xl bg-[rgba(255,255,255,0.08)] border border-[var(--glass-border)] text-[var(--text-primary)] text-[0.85rem] focus:outline-none focus:border-[var(--accent-primary)]"
            >
              {devices.map(device => (
                <option key={device.id} value={device.id}>
                  {device.name} {device.is_default ? '(Par defaut)' : ''}
                </option>
              ))}
            </select>
          </div>

          <div className="w-full max-w-sm">
            <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block text-left">
              Test du microphone
            </label>
            <div className="glass-card p-4 flex items-center justify-center">
              <AudioWaveform active={previewActive} width={280} height={60} />
            </div>
            <p className="text-[0.7rem] text-[var(--text-muted)] mt-2">
              Parlez pour verifier que le microphone fonctionne
            </p>
          </div>
        </>
      )}

      {status === 'denied' && (
        <>
          <p className="text-[var(--accent-danger)] text-[0.85rem] mb-4">
            L'acces au microphone est requis pour la dictee vocale.
          </p>
          <p className="text-[var(--text-muted)] text-[0.75rem] mb-4">
            Autorisez Phonon dans Reglages Systeme &gt; Confidentialite et securite &gt; Microphone
          </p>
          <button onClick={checkMicrophoneAccess} className="btn-glass">
            Reessayer
          </button>
        </>
      )}
    </div>
  );
}
