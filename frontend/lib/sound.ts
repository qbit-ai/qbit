/**
 * Audio playback utility for notification sounds.
 *
 * Uses the Web Audio API for reliable cross-platform sound playback,
 * independent of the OS notification system. Generates pleasant notification
 * tones programmatically without requiring external audio files.
 */

import { logger } from "./logger";

// =============================================================================
// Types
// =============================================================================

export type NotificationSoundType = "agent" | "command";

// =============================================================================
// State
// =============================================================================

/** Shared AudioContext instance (created on first use) */
let audioContext: AudioContext | null = null;

/** Default volume for notification sounds (0.0 to 1.0) */
let currentVolume = 0.5;

// =============================================================================
// Audio Context Management
// =============================================================================

/**
 * Get or create the AudioContext.
 * AudioContext must be created after a user gesture on some browsers.
 */
function getAudioContext(): AudioContext {
  if (!audioContext) {
    audioContext = new AudioContext();
  }

  // Resume if suspended (can happen due to autoplay policies)
  if (audioContext.state === "suspended") {
    audioContext.resume().catch((err) => {
      logger.warn("Failed to resume AudioContext:", err);
    });
  }

  return audioContext;
}

// =============================================================================
// Sound Generation
// =============================================================================

/**
 * Play a two-tone notification sound.
 * Creates a pleasant ascending or descending tone pattern.
 */
function playTone(frequencies: number[], durations: number[], volume: number): void {
  const ctx = getAudioContext();
  const now = ctx.currentTime;

  let timeOffset = 0;

  for (let i = 0; i < frequencies.length; i++) {
    const freq = frequencies[i];
    const duration = durations[i];

    // Create oscillator for this tone
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();

    oscillator.connect(gainNode);
    gainNode.connect(ctx.destination);

    // Use a soft sine wave
    oscillator.type = "sine";
    oscillator.frequency.setValueAtTime(freq, now + timeOffset);

    // Envelope: quick attack, sustain, quick release
    const attackTime = 0.02;
    const releaseTime = 0.08;

    gainNode.gain.setValueAtTime(0, now + timeOffset);
    gainNode.gain.linearRampToValueAtTime(volume, now + timeOffset + attackTime);
    gainNode.gain.setValueAtTime(volume, now + timeOffset + duration - releaseTime);
    gainNode.gain.linearRampToValueAtTime(0, now + timeOffset + duration);

    oscillator.start(now + timeOffset);
    oscillator.stop(now + timeOffset + duration);

    timeOffset += duration;
  }
}

/**
 * Sound configurations for different notification types.
 */
const SOUND_CONFIGS: Record<NotificationSoundType, { frequencies: number[]; durations: number[] }> =
  {
    // Agent completion: ascending two-tone (achievement/success feel)
    agent: {
      frequencies: [523.25, 659.25], // C5, E5 - major third interval
      durations: [0.12, 0.18],
    },
    // Command completion: single soft tone (subtle acknowledgment)
    command: {
      frequencies: [587.33], // D5
      durations: [0.15],
    },
  };

// =============================================================================
// Public API
// =============================================================================

/**
 * Play a notification sound.
 *
 * @param type - The type of notification sound to play
 */
export function playNotificationSound(type: NotificationSoundType): void {
  try {
    const config = SOUND_CONFIGS[type];
    playTone(config.frequencies, config.durations, currentVolume);
    logger.debug(`Played notification sound: ${type}`);
  } catch (error) {
    // Audio playback can fail for various reasons (autoplay policy, no audio device, etc.)
    // Log but don't throw - sound is a nice-to-have, not critical
    logger.warn(`Failed to play notification sound (${type}):`, error);
  }
}

/**
 * Play a test sound for the settings UI.
 * Uses the agent sound for testing as it's the primary notification.
 */
export function playTestSound(): void {
  playNotificationSound("agent");
}

/**
 * Initialize the audio system.
 * Call this after a user gesture to ensure AudioContext can be created.
 */
export function initAudio(): void {
  try {
    getAudioContext();
    logger.debug("Audio system initialized");
  } catch (error) {
    logger.warn("Failed to initialize audio system:", error);
  }
}

/**
 * Set the volume for all notification sounds.
 *
 * @param volume - Volume level from 0.0 (silent) to 1.0 (full)
 */
export function setNotificationVolume(volume: number): void {
  currentVolume = Math.max(0, Math.min(1, volume));
}

/**
 * Get the current notification volume.
 */
export function getNotificationVolume(): number {
  return currentVolume;
}
