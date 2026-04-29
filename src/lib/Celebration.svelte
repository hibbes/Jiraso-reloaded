<script lang="ts">
  import { celebration } from './celebration.svelte.ts';
  import { onDestroy } from 'svelte';

  let canvas: HTMLCanvasElement | undefined = $state();
  let raf: number | undefined;

  type Star = {
    x: number; y: number;
    vx: number; vy: number;
    rot: number; vrot: number;
    size: number; emoji: string;
    life: number; // 0..1, fades out
  };
  let stars: Star[] = [];

  function start() {
    if (!canvas) return;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    const origin = celebration.origin;
    if (origin) {
      // Firework-Burst vom Knopf aus: alle Sterne starten am gleichen Punkt
      // mit Geschwindigkeit in alle Richtungen
      stars = Array.from({ length: 80 }, () => {
        const angle = Math.random() * Math.PI * 2;
        const speed = 4 + Math.random() * 8;
        return {
          x: origin.x,
          y: origin.y,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed - 2, // leicht nach oben
          rot: Math.random() * Math.PI * 2,
          vrot: (Math.random() - 0.5) * 0.3,
          size: 14 + Math.random() * 18,
          emoji: ['★', '✦', '✧', '⭐', '✨'][Math.floor(Math.random() * 5)],
          life: 1,
        };
      });
    } else {
      // Klassischer Regen von oben
      stars = Array.from({ length: 120 }, () => ({
        x: Math.random() * canvas!.width,
        y: -20 - Math.random() * 200,
        vx: (Math.random() - 0.5) * 2,
        vy: 2 + Math.random() * 4,
        rot: Math.random() * Math.PI * 2,
        vrot: (Math.random() - 0.5) * 0.2,
        size: 16 + Math.random() * 16,
        emoji: ['★', '✦', '✧', '⭐'][Math.floor(Math.random() * 4)],
        life: 1,
      }));
    }
    tick();
  }

  function tick() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const burst = celebration.origin !== null;
    for (const s of stars) {
      s.x += s.vx;
      s.y += s.vy;
      s.rot += s.vrot;
      if (burst) {
        s.vy += 0.15; // Gravitation für Burst-Sterne
        s.vx *= 0.99;
        s.life = Math.max(0, s.life - 0.012);
      }
      ctx.save();
      ctx.translate(s.x, s.y);
      ctx.rotate(s.rot);
      ctx.font = `${s.size}px sans-serif`;
      ctx.fillStyle = burst
        ? `rgba(255, 213, 102, ${s.life})`
        : '#ffd566';
      ctx.fillText(s.emoji, -s.size / 2, s.size / 2);
      ctx.restore();
    }
    if (celebration.active) raf = requestAnimationFrame(tick);
  }

  $effect(() => {
    if (celebration.active) start();
    else if (raf) { cancelAnimationFrame(raf); raf = undefined; }
  });

  onDestroy(() => { if (raf) cancelAnimationFrame(raf); });
</script>

{#if celebration.active}
  <canvas bind:this={canvas} class="celebration"></canvas>
{/if}

<style>
  .celebration {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 9999;
  }
</style>
