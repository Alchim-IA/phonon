use crate::types::ModelSize;
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct ModelManager {
    models_dir: PathBuf,
    bundled_model_path: Option<PathBuf>,
}

impl ModelManager {
    pub fn new(app_data_dir: PathBuf, bundled_model_path: Option<PathBuf>) -> Self {
        let models_dir = app_data_dir.join("models");
        Self {
            models_dir,
            bundled_model_path,
        }
    }

    /// Retourne le chemin du modèle s'il existe
    pub fn get_model_path(&self, size: ModelSize) -> Option<PathBuf> {
        // Pour tiny, vérifier d'abord le bundled
        if size == ModelSize::Tiny {
            if let Some(ref bundled) = self.bundled_model_path {
                let bundled_model = bundled.join(size.file_name());
                if bundled_model.exists() {
                    return Some(bundled_model);
                }
            }
        }

        // Sinon, vérifier dans le dossier utilisateur
        let user_model = self.models_dir.join(size.file_name());
        if user_model.exists() {
            Some(user_model)
        } else {
            None
        }
    }

    /// Vérifie si un modèle est disponible
    pub fn is_model_available(&self, size: ModelSize) -> bool {
        self.get_model_path(size).is_some()
    }

    /// Liste les modèles disponibles
    pub fn available_models(&self) -> Vec<ModelSize> {
        [ModelSize::Tiny, ModelSize::Small, ModelSize::Medium]
            .into_iter()
            .filter(|&size| self.is_model_available(size))
            .collect()
    }

    /// Télécharge un modèle depuis Hugging Face
    pub async fn download_model<F>(
        &self,
        size: ModelSize,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(u64, u64) + Send + 'static,
    {
        // Créer le dossier models si nécessaire
        fs::create_dir_all(&self.models_dir)
            .await
            .map_err(|e| format!("Failed to create models directory: {}", e))?;

        let dest_path = self.models_dir.join(size.file_name());
        let url = size.download_url();

        log::info!("Downloading model {} from {}", size.file_name(), url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to start download: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(size.size_bytes());
        let mut downloaded: u64 = 0;

        let mut file = fs::File::create(&dest_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;
            downloaded += chunk.len() as u64;
            progress_callback(downloaded, total_size);
        }

        file.flush()
            .await
            .map_err(|e| format!("Flush error: {}", e))?;

        log::info!("Model {} downloaded successfully", size.file_name());
        Ok(dest_path)
    }

    /// Supprime un modèle téléchargé
    pub async fn delete_model(&self, size: ModelSize) -> Result<(), String> {
        let path = self.models_dir.join(size.file_name());
        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| format!("Failed to delete model: {}", e))?;
        }
        Ok(())
    }
}
