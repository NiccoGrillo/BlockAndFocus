<script lang="ts">
  import StatusPanel from "./lib/components/StatusPanel.svelte";
  import BlocklistEditor from "./lib/components/BlocklistEditor.svelte";
  import ScheduleEditor from "./lib/components/ScheduleEditor.svelte";
  import QuizModal from "./lib/components/QuizModal.svelte";

  let activeTab = $state<"status" | "blocklist" | "schedule">("status");
  let showQuiz = $state(false);
  let quizData = $state<{ challengeId: string; questions: string[]; expiresAt: number } | null>(null);

  function openQuiz(data: { challengeId: string; questions: string[]; expiresAt: number }) {
    quizData = data;
    showQuiz = true;
  }

  function closeQuiz() {
    showQuiz = false;
    quizData = null;
  }
</script>

<main>
  <header>
    <h1>BlockAndFocus</h1>
    <nav>
      <button class:active={activeTab === "status"} onclick={() => (activeTab = "status")}>
        Status
      </button>
      <button class:active={activeTab === "blocklist"} onclick={() => (activeTab = "blocklist")}>
        Blocklist
      </button>
      <button class:active={activeTab === "schedule"} onclick={() => (activeTab = "schedule")}>
        Schedule
      </button>
    </nav>
  </header>

  <section class="content">
    {#if activeTab === "status"}
      <StatusPanel onRequestBypass={openQuiz} />
    {:else if activeTab === "blocklist"}
      <BlocklistEditor />
    {:else if activeTab === "schedule"}
      <ScheduleEditor />
    {/if}
  </section>

  {#if showQuiz && quizData}
    <QuizModal
      challengeId={quizData.challengeId}
      questions={quizData.questions}
      expiresAt={quizData.expiresAt}
      onClose={closeQuiz}
    />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen,
      Ubuntu, Cantarell, sans-serif;
    background: #1a1a2e;
    color: #eee;
  }

  main {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  header {
    background: #16213e;
    padding: 1rem;
    border-bottom: 1px solid #0f3460;
  }

  h1 {
    margin: 0 0 0.75rem 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: #e94560;
  }

  nav {
    display: flex;
    gap: 0.5rem;
  }

  nav button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    background: #0f3460;
    color: #ccc;
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.2s;
  }

  nav button:hover {
    background: #1a4a7a;
  }

  nav button.active {
    background: #e94560;
    color: white;
  }

  .content {
    flex: 1;
    padding: 1rem;
    overflow-y: auto;
  }
</style>
