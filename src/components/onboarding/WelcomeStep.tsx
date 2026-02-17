import logoSvg from '../../assets/logo.svg';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

export function WelcomeStep(_props: StepProps) {
  return (
    <div className="flex flex-col items-center text-center py-6">
      <div className="w-24 h-24 rounded-3xl bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)] flex items-center justify-center shadow-lg mb-6 overflow-visible">
        <img src={logoSvg} alt="Phonon" className="w-96 h-96" />
      </div>

      <h1 className="font-display text-3xl tracking-tight mb-3">
        <span className="text-[var(--text-primary)]">Bienvenue sur </span>
        <span className="bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] bg-clip-text text-transparent">
          Phonon
        </span>
      </h1>

      <p className="text-[var(--text-secondary)] text-[0.95rem] mb-8 max-w-md">
        Dictee vocale locale, privee et rapide. Transformez votre voix en texte sans connexion Internet.
      </p>

      <div className="flex gap-4">
        <div className="glass-card px-5 py-3 flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-success)" strokeWidth="1.5">
            <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
          <span className="text-[0.8rem] text-[var(--text-primary)]">100% Offline</span>
        </div>
        <div className="glass-card px-5 py-3 flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" strokeWidth="1.5">
            <circle cx="12" cy="12" r="10" />
            <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
          </svg>
          <span className="text-[0.8rem] text-[var(--text-primary)]">Multi-langues</span>
        </div>
        <div className="glass-card px-5 py-3 flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-secondary)" strokeWidth="1.5">
            <path d="M12 2a4 4 0 0 1 4 4c0 1.95-1.4 3.58-3.25 3.93L12 22l-.75-12.07A4.001 4.001 0 0 1 12 2z" />
          </svg>
          <span className="text-[0.8rem] text-[var(--text-primary)]">IA integree</span>
        </div>
      </div>
    </div>
  );
}
