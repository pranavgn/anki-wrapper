/**
 * Reactive store bridging StudyView state → global navbar.
 * StudyView writes here on mount/update; App.svelte reads to render study nav controls.
 */
class StudyNavState {
  /** Whether study mode is actively running */
  active = $state(false);

  /** Current deck name */
  deckName = $state('');

  /** Cards remaining in session */
  remainingCards = $state(0);

  /** Cards reviewed this session */
  reviewedCount = $state(0);

  /** Current card's flag (0 = none, 1-7 = colors) */
  currentFlag = $state(0);

  /** Whether undo is available */
  canUndo = $state(false);

  /** Whether the flag picker popover is open */
  showFlagPicker = $state(false);

  /** Progress percentage 0-100 */
  progress = $state(0);

  /** Flag color palette */
  readonly FLAG_COLORS: Record<number, string> = {
    0: 'transparent',
    1: '#EF4444',
    2: '#F97316',
    3: '#22C55E',
    4: '#3B82F6',
    5: '#EC4899',
    6: '#14B8A6',
    7: '#8B5CF6',
  };

  /** Callback: set flag on current card. Set by StudyView on mount. */
  setFlag: ((flag: number) => void) | null = null;

  /** Callback: undo last review. Set by StudyView on mount. */
  undo: (() => void) | null = null;

  /** Callback: exit study mode. Set by StudyView on mount. */
  exit: (() => void) | null = null;

  /** Called by StudyView on mount to activate study nav */
  activate(deckName: string, exitFn: () => void) {
    this.active = true;
    this.deckName = deckName;
    this.exit = exitFn;
    this.remainingCards = 0;
    this.reviewedCount = 0;
    this.currentFlag = 0;
    this.canUndo = false;
    this.showFlagPicker = false;
    this.progress = 0;
  }

  /** Called by StudyView on destroy to deactivate study nav */
  deactivate() {
    this.active = false;
    this.deckName = '';
    this.setFlag = null;
    this.undo = null;
    this.exit = null;
    this.showFlagPicker = false;
  }
}

export const studyNav = new StudyNavState();
