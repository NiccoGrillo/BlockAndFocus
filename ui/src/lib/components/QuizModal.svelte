<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    challengeId: string;
    questions: string[];
    expiresAt: number;
    onClose: () => void;
  }

  let { challengeId, questions, expiresAt, onClose }: Props = $props();

  let answers = $state<string[]>(questions.map(() => ""));
  let timeRemaining = $state(0);
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let success = $state<string | null>(null);

  function updateTimeRemaining() {
    const now = Math.floor(Date.now() / 1000);
    timeRemaining = Math.max(0, expiresAt - now);
  }

  async function submitAnswers() {
    // Validate all answers are filled
    const numericAnswers = answers.map((a) => parseInt(a.trim()));
    if (numericAnswers.some((a) => isNaN(a))) {
      error = "Please enter a valid number for each question";
      return;
    }

    submitting = true;
    error = null;

    try {
      // @ts-ignore
      const result = await window.__TAURI__.core.invoke("submit_quiz_answers", {
        challengeId,
        answers: numericAnswers,
      });

      if (result.success) {
        success = result.message;
        setTimeout(onClose, 1500);
      } else {
        error = result.message;
      }
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "Enter") {
      if (index < questions.length - 1) {
        // Focus next input
        const inputs = document.querySelectorAll<HTMLInputElement>(".answer-input");
        inputs[index + 1]?.focus();
      } else {
        // Submit on last question
        submitAnswers();
      }
    }
  }

  onMount(() => {
    updateTimeRemaining();

    const timer = setInterval(() => {
      updateTimeRemaining();
      if (timeRemaining <= 0) {
        clearInterval(timer);
        error = "Time's up! Quiz expired.";
        setTimeout(onClose, 2000);
      }
    }, 1000);

    // Focus first input
    setTimeout(() => {
      const inputs = document.querySelectorAll<HTMLInputElement>(".answer-input");
      inputs[0]?.focus();
    }, 100);

    return () => clearInterval(timer);
  });

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }
</script>

<div class="modal-backdrop" onclick={onClose}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>Solve to Bypass</h2>
      <div class="timer" class:urgent={timeRemaining <= 10}>
        {formatTime(timeRemaining)}
      </div>
    </div>

    {#if success}
      <div class="success">{success}</div>
    {:else}
      <div class="questions">
        {#each questions as question, i}
          <div class="question-row">
            <span class="question">{question}</span>
            <input
              type="text"
              inputmode="numeric"
              class="answer-input"
              bind:value={answers[i]}
              onkeydown={(e) => handleKeydown(e, i)}
              disabled={submitting}
              placeholder="?"
            />
          </div>
        {/each}
      </div>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      <div class="actions">
        <button class="btn-secondary" onclick={onClose} disabled={submitting}>
          Cancel
        </button>
        <button class="btn-primary" onclick={submitAnswers} disabled={submitting}>
          {submitting ? "Checking..." : "Submit"}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: #1a1a2e;
    border: 1px solid #0f3460;
    border-radius: 12px;
    padding: 1.5rem;
    width: 90%;
    max-width: 350px;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  h2 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
  }

  .timer {
    font-family: monospace;
    font-size: 1.25rem;
    font-weight: bold;
    color: #4caf50;
    padding: 0.25rem 0.75rem;
    background: rgba(76, 175, 80, 0.2);
    border-radius: 4px;
  }

  .timer.urgent {
    color: #dc3545;
    background: rgba(220, 53, 69, 0.2);
  }

  .questions {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .question-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .question {
    font-family: monospace;
    font-size: 1.125rem;
  }

  .answer-input {
    width: 80px;
    padding: 0.75rem;
    border: 2px solid #0f3460;
    border-radius: 6px;
    background: #16213e;
    color: #eee;
    font-size: 1.125rem;
    font-family: monospace;
    text-align: center;
  }

  .answer-input:focus {
    outline: none;
    border-color: #e94560;
  }

  .answer-input::placeholder {
    color: #444;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  button {
    padding: 0.75rem 1.25rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #e94560;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #d63b55;
  }

  .btn-secondary {
    background: #0f3460;
    color: #ccc;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #1a4a7a;
  }

  .error {
    background: #3d1f1f;
    border: 1px solid #dc3545;
    border-radius: 8px;
    padding: 0.75rem;
    color: #ff6b6b;
    font-size: 0.875rem;
    margin-bottom: 1rem;
    text-align: center;
  }

  .success {
    background: #1f3d1f;
    border: 1px solid #4caf50;
    border-radius: 8px;
    padding: 1.5rem;
    color: #6eff6e;
    font-size: 1rem;
    text-align: center;
  }
</style>
