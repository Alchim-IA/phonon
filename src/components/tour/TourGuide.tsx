import { useState, useEffect, useCallback } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';

interface TourStep {
  target: string;
  title: string;
  description: string;
}

const TOUR_STEPS: TourStep[] = [
  {
    target: 'tour-header',
    title: 'En-tete',
    description: 'Le logo WakaScribe et l\'indicateur de statut. Le point vert signifie que le systeme est pret.',
  },
  {
    target: 'tour-nav',
    title: 'Navigation',
    description: 'Basculez entre la Dictee en direct, l\'Historique de vos transcriptions, et la transcription de Fichiers audio.',
  },
  {
    target: 'tour-main',
    title: 'Zone principale',
    description: 'C\'est ici que la magie opere ! Utilisez le raccourci Push-to-talk pour commencer a dicter.',
  },
  {
    target: 'tour-footer',
    title: 'Barre d\'infos',
    description: 'Le moteur actif et le raccourci Push-to-talk sont affiches ici. Personnalisez-les dans les Parametres.',
  },
];

const SPOTLIGHT_PADDING = 12;
const TOOLTIP_GAP = 16;

export function TourGuide() {
  const { updateSettings } = useSettingsStore();
  const [currentStep, setCurrentStep] = useState(0);
  const [rect, setRect] = useState<DOMRect | null>(null);

  const step = TOUR_STEPS[currentStep];

  const measureTarget = useCallback(() => {
    const el = document.querySelector(`[data-tour="${step.target}"]`);
    if (el) {
      setRect(el.getBoundingClientRect());
    }
  }, [step.target]);

  useEffect(() => {
    measureTarget();
    window.addEventListener('resize', measureTarget);
    return () => window.removeEventListener('resize', measureTarget);
  }, [measureTarget]);

  const handleNext = () => {
    if (currentStep < TOUR_STEPS.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      handleFinish();
    }
  };

  const handleFinish = async () => {
    await updateSettings({ tour_completed: true });
  };

  if (!rect) return null;

  // Spotlight coordinates with padding
  const sx = rect.left - SPOTLIGHT_PADDING;
  const sy = rect.top - SPOTLIGHT_PADDING;
  const sw = rect.width + SPOTLIGHT_PADDING * 2;
  const sh = rect.height + SPOTLIGHT_PADDING * 2;
  const sr = 16; // border-radius

  // Tooltip position: try below, then above
  const tooltipWidth = 320;
  const spaceBelow = window.innerHeight - (sy + sh);
  const placeBelow = spaceBelow > 160;

  const tooltipStyle: React.CSSProperties = {
    position: 'fixed',
    width: tooltipWidth,
    left: Math.min(Math.max(sx, 16), window.innerWidth - tooltipWidth - 16),
    ...(placeBelow
      ? { top: sy + sh + TOOLTIP_GAP }
      : { bottom: window.innerHeight - sy + TOOLTIP_GAP }),
    zIndex: 10001,
  };

  return (
    <div className="fixed inset-0" style={{ zIndex: 10000 }}>
      {/* SVG mask overlay */}
      <svg className="absolute inset-0 w-full h-full">
        <defs>
          <mask id="tour-mask">
            <rect width="100%" height="100%" fill="white" />
            <rect
              x={sx} y={sy} width={sw} height={sh}
              rx={sr} ry={sr}
              fill="black"
            />
          </mask>
        </defs>
        <rect
          width="100%" height="100%"
          fill="rgba(0,0,0,0.65)"
          mask="url(#tour-mask)"
        />
        {/* Spotlight border */}
        <rect
          x={sx} y={sy} width={sw} height={sh}
          rx={sr} ry={sr}
          fill="none"
          stroke="var(--accent-primary)"
          strokeWidth="2"
          opacity="0.6"
        />
      </svg>

      {/* Tooltip */}
      <div style={tooltipStyle}>
        <div className="glass-panel p-5 shadow-2xl border-[var(--accent-primary)] border">
          {/* Step counter */}
          <div className="flex items-center justify-between mb-3">
            <span className="text-[0.7rem] text-[var(--text-muted)]">
              {currentStep + 1} / {TOUR_STEPS.length}
            </span>
            <div className="flex gap-1">
              {TOUR_STEPS.map((_, i) => (
                <div
                  key={i}
                  className={`w-1.5 h-1.5 rounded-full transition-all ${
                    i === currentStep
                      ? 'bg-[var(--accent-primary)] w-4'
                      : i < currentStep
                      ? 'bg-[var(--accent-success)]'
                      : 'bg-[var(--glass-border)]'
                  }`}
                />
              ))}
            </div>
          </div>

          <h3 className="text-[0.95rem] font-display text-[var(--text-primary)] mb-2">
            {step.title}
          </h3>
          <p className="text-[0.8rem] text-[var(--text-secondary)] leading-relaxed mb-4">
            {step.description}
          </p>

          <div className="flex justify-between items-center">
            <button
              onClick={handleFinish}
              className="text-[0.75rem] text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
            >
              Passer
            </button>
            <button
              onClick={handleNext}
              className="btn-glass text-[0.8rem]"
            >
              {currentStep < TOUR_STEPS.length - 1 ? 'Suivant' : 'Terminer'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
