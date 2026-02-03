# PRD WakaScribe - Document de Référence pour Claude Code

**Version:** 2.0 | **Date:** 30 janvier 2026 | **Propriétaire:** Denis SCHIRRA  
**Technologie:** Tauri 2.x + Rust + TypeScript/React

---

## 1. Vue d'Ensemble

### 1.1 Description
WakaScribe est une application de dictée vocale cross-platform (macOS/Windows) fonctionnant en mode local sans connexion Internet. Utilise le moteur NVIDIA Parakeet v3 avec optimisations matérielles.

### 1.2 Distribution
- **Type:** Freeware gratuit
- **Canaux:** Site WakaStellar, App Store, Microsoft Store
- **Auth:** Compte obligatoire via app.wakascribe.com (validation mensuelle)

### 1.3 Réseaux
- **WakaScribe → Europe:** Inscription gratuite, usage illimité, Teams, Groq/LLM
- **WakaClub → {membre}:** Via WakaStart, gratuit pour membres

---

## 2. Architecture Technique

### 2.1 Stack
```
FRONTEND: React 18 + TypeScript + TailwindCSS + Zustand
BACKEND: Rust/Tauri 2.x + cpal + reqwest + keyring
MOTEURS STT: Parakeet MLX (Apple) | OpenVINO (Intel) | TensorRT (NVIDIA)
```

### 2.2 Structure Projet
```
wakascribe/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/{transcription,audio,auth,settings}.rs
│   │   ├── engines/{selector,mlx,openvino,tensorrt}.rs
│   │   ├── audio/{capture,buffer,vad}.rs
│   │   ├── api/{client,auth,sync}.rs
│   │   └── storage/{config,dictionary}.rs
│   ├── binaries/parakeet-*-engine
│   └── models/parakeet-{mlx,openvino}/
├── src/
│   ├── components/{dictation,settings,auth}/
│   ├── stores/{transcription,settings,auth}Store.ts
│   ├── hooks/
│   └── types/
└── package.json
```

---

## 3. Moteur Speech-to-Text

### 3.1 Matrice de Sélection
| Architecture | Framework | Performance | Modèle |
|-------------|-----------|-------------|--------|
| Apple Silicon | MLX | 10-15x RTF | parakeet-mlx |
| Intel 11e+ | OpenVINO | 4-8x RTF | parakeet-tdt-0.6b-v3-ov |
| NVIDIA RTX | TensorRT | 100-200x RTF | NIM Container |
| Fallback | ONNX | 1-3x RTF | parakeet-onnx |

### 3.2 Sélecteur de Moteur (Rust)
```rust
pub fn select_engine(hw: &HardwareInfo) -> EngineType {
    if hw.has_apple_silicon { return EngineType::MLX; }
    if hw.has_nvidia_gpu && hw.nvidia_compute_capability >= Some(7.0) {
        return EngineType::TensorRT;
    }
    if hw.cpu_brand.contains("Intel") { return EngineType::OpenVINO; }
    EngineType::ONNXFallback
}
```

### 3.3 Intégration MLX (Sidecar)
```rust
#[tauri::command]
pub async fn transcribe_mlx(app: tauri::AppHandle, audio_path: String) -> Result<TranscriptionResult, String> {
    let output = app.shell()
        .sidecar("parakeet-mlx-engine")
        .map_err(|e| e.to_string())?
        .arg(&audio_path)
        .output().await
        .map_err(|e| e.to_string())?;
    parse_transcription_json(&output.stdout)
}
```

### 3.4 Intégration OpenVINO
```rust
use openvino::{Core, Tensor};

pub struct OpenVINOEngine {
    compiled_model: openvino::CompiledModel,
}

impl OpenVINOEngine {
    pub fn new(model_path: &str) -> Result<Self, Error> {
        let core = Core::new()?;
        let model = core.read_model(model_path)?;
        let compiled = core.compile_model(&model, "AUTO")?;
        Ok(Self { compiled_model: compiled })
    }
    
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, Error> {
        let mut req = self.compiled_model.create_infer_request()?;
        let tensor = Tensor::new(openvino::ElementType::F32, &[1, audio.len()], audio)?;
        req.set_input_tensor(&tensor)?;
        req.infer()?;
        self.decode_output(&req.get_output_tensor()?)
    }
}
```

---

## 4. Backend Rust

### 4.1 Dépendances (Cargo.toml)
```toml
[dependencies]
tauri = { version = "2.0", features = ["tray-icon"] }
tauri-plugin-shell = "2.0"
tauri-plugin-global-shortcut = "2.0"
tauri-plugin-notification = "2.0"
tauri-plugin-clipboard-manager = "2.0"
tauri-plugin-updater = "2.0"
tokio = { version = "1.35", features = ["full"] }
cpal = "0.15"
hound = "3.5"
openvino = { version = "0.5", optional = true }
sysinfo = "0.30"
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
keyring = "2.2"
```

### 4.2 État Application
```rust
pub struct AppState {
    pub hardware: HardwareInfo,
    pub engine_type: EngineType,
    pub is_recording: Arc<RwLock<bool>>,
    pub is_online: Arc<RwLock<bool>>,
    pub auth_token: Arc<RwLock<Option<String>>>,
}
```

### 4.3 Commandes Transcription
```rust
#[derive(Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub duration_seconds: f32,
    pub processing_time_ms: u64,
    pub detected_language: Option<String>,
}

#[tauri::command]
pub async fn start_recording(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.is_recording.write();
    if *recording { return Err("Already recording".into()); }
    AudioCapture::start(&app)?;
    *recording = true;
    app.emit("recording-started", ())?;
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<TranscriptionResult, String> {
    let mut recording = state.is_recording.write();
    if !*recording { return Err("Not recording".into()); }
    let audio = AudioCapture::stop(&app)?;
    *recording = false;
    transcribe_audio_data(&app, &state, audio).await
}
```

### 4.4 Capture Audio
```rust
pub struct AudioCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl AudioCapture {
    pub fn list_devices() -> Result<Vec<AudioDevice>, Error> {
        let host = cpal::default_host();
        host.input_devices()?.filter_map(|d| {
            d.name().ok().map(|name| AudioDevice { id: name.clone(), name, ..Default::default() })
        }).collect()
    }
    
    pub fn start(device_id: Option<&str>) -> Result<Self, Error> {
        let host = cpal::default_host();
        let device = device_id
            .and_then(|id| host.input_devices().ok()?.find(|d| d.name().ok() == Some(id.into())))
            .or_else(|| host.default_input_device())
            .ok_or(Error::NoDevice)?;
        
        let config = device.default_input_config()?;
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buf_clone = buffer.clone();
        
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| buf_clone.lock().extend_from_slice(data),
            |e| eprintln!("Audio error: {}", e),
            None,
        )?;
        stream.play()?;
        
        Ok(Self { buffer, sample_rate: config.sample_rate().0 })
    }
}
```

### 4.5 Authentification
```rust
const API_BASE: &str = "https://app.wakascribe.com/api/v1";

pub struct AuthClient { client: reqwest::Client }

impl AuthClient {
    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, Error> {
        let resp = self.client.post(format!("{}/auth/login", API_BASE))
            .json(&json!({ "email": email, "password": password }))
            .send().await?;
        let data: LoginResponse = resp.json().await?;
        keyring::Entry::new("wakascribe", "access_token")?.set_password(&data.access_token)?;
        keyring::Entry::new("wakascribe", "refresh_token")?.set_password(&data.refresh_token)?;
        Ok(data)
    }
    
    pub async fn refresh_token(&self) -> Result<LoginResponse, Error> {
        let token = keyring::Entry::new("wakascribe", "refresh_token")?.get_password()?;
        let resp = self.client.post(format!("{}/auth/refresh", API_BASE))
            .bearer_auth(&token).send().await?;
        let data: LoginResponse = resp.json().await?;
        // Store new tokens...
        Ok(data)
    }
}
```

---

## 5. Frontend React/TypeScript

### 5.1 Types
```typescript
interface TranscriptionResult {
  text: string;
  confidence: number;
  duration_seconds: number;
  processing_time_ms: number;
  detected_language: string | null;
}

interface AppSettings {
  microphone_id: string | null;
  hotkey_push_to_talk: string;
  hotkey_toggle_record: string;
  transcription_language: string;
  auto_copy_to_clipboard: boolean;
  llm_enabled: boolean;
  theme: 'light' | 'dark' | 'system';
}
```

### 5.2 Store Transcription (Zustand)
```typescript
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface TranscriptionStore {
  status: 'idle' | 'recording' | 'processing' | 'completed' | 'error';
  result: TranscriptionResult | null;
  history: TranscriptionResult[];
  startRecording: () => Promise<void>;
  stopRecording: () => Promise<TranscriptionResult>;
}

export const useTranscriptionStore = create<TranscriptionStore>((set) => ({
  status: 'idle',
  result: null,
  history: [],
  
  startRecording: async () => {
    set({ status: 'recording' });
    await invoke('start_recording');
  },
  
  stopRecording: async () => {
    set({ status: 'processing' });
    const result = await invoke<TranscriptionResult>('stop_recording');
    set(s => ({ status: 'completed', result, history: [result, ...s.history].slice(0, 50) }));
    return result;
  },
}));
```

### 5.3 Composant Dictée
```tsx
import { useTranscriptionStore } from '../stores/transcriptionStore';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Mic, MicOff, Loader2 } from 'lucide-react';

export function DictationPanel() {
  const { status, result, startRecording, stopRecording } = useTranscriptionStore();
  
  const handleToggle = async () => {
    if (status === 'recording') {
      const r = await stopRecording();
      await writeText(r.text);
    } else {
      await startRecording();
    }
  };
  
  return (
    <div className="flex flex-col items-center p-6 space-y-6">
      <button
        onClick={handleToggle}
        disabled={status === 'processing'}
        className={`w-24 h-24 rounded-full flex items-center justify-center shadow-lg
          ${status === 'recording' ? 'bg-red-500 animate-pulse' : 'bg-blue-500'}
          ${status === 'processing' ? 'bg-gray-400' : ''}`}
      >
        {status === 'processing' ? <Loader2 className="w-12 h-12 text-white animate-spin" />
         : status === 'recording' ? <MicOff className="w-12 h-12 text-white" />
         : <Mic className="w-12 h-12 text-white" />}
      </button>
      
      {result && (
        <div className="bg-white rounded-lg border p-4 w-full max-w-md">
          <p className="text-gray-900">{result.text}</p>
          <p className="text-xs text-gray-500 mt-2">
            {result.processing_time_ms}ms • {(result.confidence * 100).toFixed(0)}%
          </p>
        </div>
      )}
    </div>
  );
}
```

---

## 6. Fonctionnalités

### 6.1 Mode Offline
| Fonction | Dispo | Notes |
|----------|-------|-------|
| Dictée STT | ✅ | Parakeet local |
| Microphone | ✅ | Config locale |
| Hotkeys | ✅ | Config locale |
| Clipboard | ✅ | Auto si activé |
| Historique | ✅ | 50 entrées |
| LLM | ❌ | Requiert connexion |
| Sync | ❌ | Requiert connexion |

### 6.2 Mode LLM (Groq)
```rust
const GROQ_API: &str = "https://api.groq.com/openai/v1/chat/completions";

pub async fn enhance_transcription(text: &str, context: Option<&str>) -> Result<String, Error> {
    let prompt = format!(
        "Corrige cette transcription vocale. Ajoute ponctuation et corrige erreurs.\n{}{}",
        context.map(|c| format!("Contexte: {}\n", c)).unwrap_or_default(),
        text
    );
    
    let resp = reqwest::Client::new()
        .post(GROQ_API)
        .bearer_auth(&api_key)
        .json(&json!({
            "model": "llama-3.1-70b-versatile",
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": 0.3
        }))
        .send().await?;
    
    // Parse response...
    Ok(enhanced)
}
```

---

## 7. Configuration

### 7.1 Paramètres par Défaut
```rust
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            microphone_id: None,
            hotkey_push_to_talk: "CommandOrControl+Shift+Space".into(),
            hotkey_toggle_record: "CommandOrControl+Shift+R".into(),
            transcription_language: "auto".into(),
            auto_detect_language: true,
            theme: Theme::System,
            minimize_to_tray: true,
            auto_copy_to_clipboard: true,
            notification_on_complete: true,
            llm_enabled: false,
            llm_provider: LLMProvider::Groq,
        }
    }
}
```

### 7.2 Tauri Config
```json
{
  "productName": "WakaScribe",
  "identifier": "com.wakastellar.wakascribe",
  "bundle": {
    "resources": ["models/parakeet-mlx/**/*", "models/parakeet-openvino/**/*"],
    "externalBin": ["binaries/parakeet-mlx-engine", "binaries/parakeet-openvino-engine"],
    "targets": ["app", "dmg", "nsis"]
  },
  "plugins": {
    "updater": { "endpoints": ["https://releases.wakascribe.com/{{target}}/{{arch}}/{{current_version}}"] }
  }
}
```

---

## 8. Roadmap

### Phase 1 - MVP (4 semaines)
- S1: Setup Tauri 2.x, React, CI/CD
- S2: Audio capture, MLX sidecar
- S3: OpenVINO, sélecteur moteur
- S4: Hotkeys, clipboard, tests

### Phase 2 - Auth (3 semaines)
- S5: API client, keyring, login
- S6: Sync settings, dictionnaire
- S7: Mode LLM Groq

### Phase 3 - Distribution (2 semaines)
- S8: Auto-updater, notifications
- S9: Packaging, stores

---

## 9. Performances

| Config | RTF | Latence |
|--------|-----|---------|
| M1 Mac | 12x | <200ms |
| M2 MacBook Pro | 15x | <150ms |
| Intel i7 + OpenVINO | 6x | <400ms |
| RTX 3080 | 150x | <50ms |

---

## 10. Ressources

- **Modèles:** senstella/parakeet-mlx, FluidInference/parakeet-tdt-0.6b-v3-ov
- **Docs:** Tauri v2.tauri.app, MLX ml-explore.github.io, OpenVINO docs.openvino.ai

---

**Pour Claude Code:** Ce PRD contient toutes les spécifications nécessaires pour implémenter WakaScribe. Chaque section inclut du code fonctionnel prêt à être adapté.

clé groq : REDACTED_GROQ_KEY