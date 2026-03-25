// src/lib/loadingTracker.ts
// Tracks initialization steps and exposes real progress as a percentage.

type StepStatus = 'pending' | 'running' | 'done' | 'error';

interface LoadStep {
  name: string;
  weight: number; // relative weight (heavier = more of the bar)
  status: StepStatus;
}

class LoadingTracker {
  private steps: LoadStep[] = [];
  private _progress = $state(0);
  private _currentStep = $state('');
  private _done = $state(false);
  private _error = $state<string | null>(null);

  get progress() { return this._progress; }
  get currentStep() { return this._currentStep; }
  get done() { return this._done; }
  get error() { return this._error; }

  /** Register all steps up front before starting. */
  register(steps: { name: string; weight?: number }[]) {
    this.steps = steps.map(s => ({
      name: s.name,
      weight: s.weight ?? 1,
      status: 'pending' as StepStatus,
    }));
    this.recalc();
  }

  /** Mark a step as currently running. */
  start(name: string) {
    const step = this.steps.find(s => s.name === name);
    if (step) {
      step.status = 'running';
      this._currentStep = name;
      this.recalc();
    }
  }

  /** Mark a step as complete. */
  finish(name: string) {
    const step = this.steps.find(s => s.name === name);
    if (step) {
      step.status = 'done';
      this.recalc();
    }
  }

  /** Mark a step as errored (non-fatal — loading continues). */
  fail(name: string, error?: string) {
    const step = this.steps.find(s => s.name === name);
    if (step) {
      step.status = 'error';
      if (error) this._error = error;
      this.recalc();
    }
  }

  /** Mark a fatal error that stops loading entirely. */
  fatal(error: string) {
    this._error = error;
    this._done = true;
  }

  private recalc() {
    const totalWeight = this.steps.reduce((s, st) => s + st.weight, 0);
    if (totalWeight === 0) { this._progress = 0; return; }

    let completed = 0;
    for (const step of this.steps) {
      if (step.status === 'done' || step.status === 'error') {
        completed += step.weight;
      } else if (step.status === 'running') {
        completed += step.weight * 0.3; // partial credit while running
      }
    }
    this._progress = Math.round((completed / totalWeight) * 100);

    // Check if all done
    if (this.steps.every(s => s.status === 'done' || s.status === 'error')) {
      this._progress = 100;
      this._done = true;
    }
  }
}

export const loader = new LoadingTracker();
