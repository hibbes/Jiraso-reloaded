class CelebrationState {
  active = $state(false);
  trigger() {
    this.active = true;
    setTimeout(() => { this.active = false; }, 2500);
  }
}
export const celebration = new CelebrationState();
