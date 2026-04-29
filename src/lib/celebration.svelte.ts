type Origin = { x: number; y: number } | null;

class CelebrationState {
  active = $state(false);
  origin = $state<Origin>(null);
  trigger(origin: Origin = null) {
    this.origin = origin;
    this.active = true;
    setTimeout(() => { this.active = false; this.origin = null; }, 2500);
  }
}
export const celebration = new CelebrationState();
