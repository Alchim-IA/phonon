import Foundation
import FluidAudio

struct TranscriptionRequest: Codable {
    let audioPath: String?
    let command: String?
}

struct TranscriptionOutput: Codable {
    let text: String
    let confidence: Double
    let processingTimeMs: Int64
    let error: String?
    let command: String?
}

func outputJSON(_ output: TranscriptionOutput) {
    let encoder = JSONEncoder()
    encoder.keyEncodingStrategy = .convertToSnakeCase
    if let data = try? encoder.encode(output), let json = String(data: data, encoding: .utf8) {
        print(json)
        fflush(stdout)
    }
}

func outputError(_ message: String) {
    outputJSON(TranscriptionOutput(text: "", confidence: 0, processingTimeMs: 0, error: message, command: nil))
}

func outputCommand(_ cmd: String) {
    outputJSON(TranscriptionOutput(text: "", confidence: 0, processingTimeMs: 0, error: nil, command: cmd))
}

@main
struct ParakeetCoreML {
    static func main() async {
        let args = CommandLine.arguments

        // Parse optional model directory argument
        var modelDir: String? = nil
        if let idx = args.firstIndex(of: "--model-dir"), idx + 1 < args.count {
            modelDir = args[idx + 1]
        }

        // Load models once at startup
        let models: AsrModels
        do {
            if let dir = modelDir {
                let modelURL = URL(fileURLWithPath: dir)
                models = try await AsrModels.load(from: modelURL)
            } else {
                models = try await AsrModels.downloadAndLoad(version: .v3)
            }
        } catch {
            outputError("Failed to load models: \(error.localizedDescription)")
            return
        }

        // Initialize ASR manager once
        let config = ASRConfig.default
        let asr = AsrManager(config: config)
        do {
            try await asr.initialize(models: models)
        } catch {
            outputError("Failed to initialize ASR: \(error.localizedDescription)")
            return
        }

        // Signal that the daemon is ready
        outputCommand("ready")

        // Read JSON requests from stdin in a loop
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase

        while let line = readLine() {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty { continue }

            guard let data = trimmed.data(using: .utf8) else {
                outputError("Invalid UTF-8 input")
                continue
            }

            let request: TranscriptionRequest
            do {
                request = try decoder.decode(TranscriptionRequest.self, from: data)
            } catch {
                outputError("Invalid JSON: \(error.localizedDescription)")
                continue
            }

            // Handle quit command
            if request.command == "quit" {
                asr.cleanup()
                return
            }

            // Handle transcription request
            guard let audioPath = request.audioPath else {
                outputError("Missing audio_path in request")
                continue
            }

            guard FileManager.default.fileExists(atPath: audioPath) else {
                outputError("Audio file not found: \(audioPath)")
                continue
            }

            let startTime = Date()

            do {
                let audioURL = URL(fileURLWithPath: audioPath)
                let result = try await asr.transcribe(audioURL, source: .system)
                let processingTime = Int64(Date().timeIntervalSince(startTime) * 1000)

                outputJSON(TranscriptionOutput(
                    text: result.text,
                    confidence: Double(result.confidence),
                    processingTimeMs: processingTime,
                    error: nil,
                    command: nil
                ))
            } catch {
                outputError("Transcription failed: \(error.localizedDescription)")
            }
        }

        // stdin closed, clean up
        asr.cleanup()
    }
}
