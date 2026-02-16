import { useState } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';
import { WelcomeStep } from './WelcomeStep';
import { PermissionStep } from './PermissionStep';
import { ModelStep } from './ModelStep';
import { LanguageStep } from './LanguageStep';
import { ShortcutsStep } from './ShortcutsStep';
import { TestStep } from './TestStep';

const STEPS = [
  { label: 'Bienvenue', component: WelcomeStep },
  { label: 'Microphone', component: PermissionStep },
  { label: 'Modele', component: ModelStep },
  { label: 'Langue', component: LanguageStep },
  { label: 'Raccourcis', component: ShortcutsStep },
  { label: 'Test', component: TestStep },
];

export function OnboardingWizard() {
  const [currentStep, setCurrentStep] = useState(0);
  const { updateSettings } = useSettingsStore();
  const [stepValid, setStepValid] = useState(true);

  const StepComponent = STEPS[currentStep].component;

  const handleNext = () => {
    if (currentStep < STEPS.length - 1) {
      setCurrentStep(currentStep + 1);
      setStepValid(true);
    }
  };

  const handleBack = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
      setStepValid(true);
    }
  };

  const handleFinish = async () => {
    await updateSettings({ onboarding_completed: true });
  };

  const isLastStep = currentStep === STEPS.length - 1;

  return (
    <div className="h-screen flex flex-col relative">
      <div className="mesh-gradient-bg" />
      <div className="noise-overlay" />

      <div className="relative z-10 flex-1 overflow-y-auto flex flex-col items-center p-6">
        <div className="flex-1" />
        {/* Stepper */}
        <div className="flex items-center gap-2 mb-8 flex-shrink-0">
          {STEPS.map((step, index) => (
            <div key={step.label} className="flex items-center gap-2">
              <div className={`w-8 h-8 rounded-full flex items-center justify-center text-[0.75rem] font-medium border transition-all ${
                index === currentStep
                  ? 'bg-[var(--accent-primary)] border-[var(--accent-primary)] text-white'
                  : index < currentStep
                  ? 'bg-[var(--accent-success)] border-[var(--accent-success)] text-white'
                  : 'bg-[rgba(255,255,255,0.08)] border-[var(--glass-border)] text-[var(--text-muted)]'
              }`}>
                {index < currentStep ? (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                ) : (
                  index + 1
                )}
              </div>
              <span className={`text-[0.7rem] hidden sm:inline ${
                index === currentStep ? 'text-[var(--text-primary)]' : 'text-[var(--text-muted)]'
              }`}>
                {step.label}
              </span>
              {index < STEPS.length - 1 && (
                <div className={`w-8 h-px ${
                  index < currentStep ? 'bg-[var(--accent-success)]' : 'bg-[var(--glass-border)]'
                }`} />
              )}
            </div>
          ))}
        </div>

        {/* Step content */}
        <div className="glass-panel w-full max-w-[700px] p-8 animate-fade-in flex-shrink-0 !overflow-visible">
          <StepComponent onValidChange={setStepValid} />
        </div>

        {/* Navigation buttons */}
        <div className="flex items-center gap-4 mt-6 flex-shrink-0">
          {currentStep > 0 && (
            <button onClick={handleBack} className="btn-glass">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M19 12H5M12 19l-7-7 7-7" />
              </svg>
              Retour
            </button>
          )}
          <button
            onClick={isLastStep ? handleFinish : handleNext}
            disabled={!stepValid}
            className={`px-6 py-2.5 rounded-xl text-[0.85rem] font-medium transition-all ${
              stepValid
                ? 'bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] text-white hover:opacity-90'
                : 'bg-[rgba(255,255,255,0.08)] text-[var(--text-muted)] cursor-not-allowed'
            }`}
          >
            {isLastStep ? 'Terminer' : 'Suivant'}
            {!isLastStep && (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="inline ml-2">
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            )}
          </button>
        </div>
        <div className="flex-1" />
      </div>
    </div>
  );
}
