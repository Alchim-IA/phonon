import Foundation
import FluidAudio

struct TranscriptionOutput: Codable {
    let text: String
    let confidence: Double
    let processingTimeMs: Int64
    let error: String?
}

func outputJSON(_ output: TranscriptionOutput) {
    let encoder = JSONEncoder()
    encoder.keyEncodingStrategy = .convertToSnakeCase
    if let data = try? encoder.encode(output), let json = String(data: data, encoding: .utf8) {
        print(json)
    }
}

func outputError(_ message: String) {
    outputJSON(TranscriptionOutput(text: "", confidence: 0, processingTimeMs: 0, error: message))
}

@main
struct ParakeetCoreML {
    static func main() async {
        let args = CommandLine.arguments

        guard args.count >= 2 else {
            outputError("Usage: parakeet-coreml <audio_file_path> [--model-dir <path>]")
            return
        }

        let audioPath = args[1]
        var modelDir: String? = nil

        // Parse optional model directory argument
        if let idx = args.firstIndex(of: "--model-dir"), idx + 1 < args.count {
            modelDir = args[idx + 1]
        }

        // Verify audio file exists
        guard FileManager.default.fileExists(atPath: audioPath) else {
            outputError("Audio file not found: \(audioPath)")
            return
        }

        let startTime = Date()

        do {
            // Download/load models (cached after first run)
            let models: AsrModels
            if let dir = modelDir {
                // Use custom model directory if specified
                let modelURL = URL(fileURLWithPath: dir)
                models = try await AsrModels.load(from: modelURL)
            } else {
                // Use default download location
                models = try await AsrModels.downloadAndLoad(version: .v3)
            }

            // Initialize ASR manager
            let config = ASRConfig.default
            let asr = AsrManager(config: config)
            try await asr.initialize(models: models)

            // Transcribe directly from URL
            let audioURL = URL(fileURLWithPath: audioPath)
            let result = try await asr.transcribe(audioURL, source: .system)

            let processingTime = Int64(Date().timeIntervalSince(startTime) * 1000)

            // Output result
            outputJSON(TranscriptionOutput(
                text: result.text,
                confidence: Double(result.confidence),
                processingTimeMs: processingTime,
                error: nil
            ))

            asr.cleanup()

        } catch {
            outputError("Transcription failed: \(error.localizedDescription)")
        }
    }
}
