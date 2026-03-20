import { prefs } from './prefs';

export interface FlyParams {
  x?: number;
  y?: number;
  duration?: number;
}

export interface FadeParams {
  duration?: number;
}

// Returns transition params or no-op params based on pref
export function fly_if_enabled(params: FlyParams): FlyParams {
  if (!prefs.animations_enabled) {
    return { duration: 0, x: 0, y: 0 };
  }
  if (prefs.reduce_motion) {
    return { duration: 80, x: 0, y: 0 };
  }
  return params;
}

export function fade_if_enabled(params: FadeParams): FadeParams {
  if (!prefs.animations_enabled) {
    return { duration: 0 };
  }
  if (prefs.reduce_motion) {
    return { duration: 80 };
  }
  return params;
}


