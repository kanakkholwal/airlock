<script lang="ts">
  import chat from "@airlock/catalog/chat-models";

  const releases = "https://github.com/kanakkholwal/airlock/releases";
  const features = [
    ["Chat", "Local models with image input. Replies stream as they are written."],
    ["Dictation", "Hold a shortcut anywhere and your words are typed for you."],
    ["Transcribe", "Turn recordings into text, with timestamps and SRT or VTT."],
    ["Read aloud", "Natural voices that run on your computer."],
    ["Images", "Describe a picture and make it on your own GPU."],
    ["Private library", "Everything you make stays here, private by default."],
  ];
</script>

<main class="mx-auto flex min-h-dvh max-w-3xl flex-col gap-16 px-6 py-24">
  <header class="flex flex-col items-start gap-6">
    <h1 class="text-5xl font-bold tracking-tight text-balance">Private AI on your own computer</h1>
    <p class="max-w-xl text-lg text-pretty text-(--muted-foreground)">
      Chat, dictate, transcribe, read aloud and make images with local models. Nothing leaves your machine unless you say so.
    </p>
    <a
      href={releases}
      class="rounded-xl bg-(--foreground) px-5 py-3 font-medium text-(--background) transition-opacity hover:opacity-90"
    >
      Download for Windows
    </a>
    <p class="text-sm text-(--muted-foreground)">Pre-release. Unsigned, so SmartScreen will ask you to confirm.</p>
  </header>

  <section class="grid gap-4 sm:grid-cols-2" aria-label="What it does">
    {#each features as [name, text] (name)}
      <div class="rounded-2xl bg-(--muted) p-5">
        <h2 class="font-medium">{name}</h2>
        <p class="mt-1 text-(--muted-foreground) text-pretty">{text}</p>
      </div>
    {/each}
  </section>

  <section class="flex flex-col gap-3">
    <h2 class="text-2xl font-bold">Models you can start with</h2>
    <ul class="flex flex-col gap-2">
      {#each chat.items as model (model.id)}
        <li class="flex items-baseline justify-between gap-4 rounded-xl bg-(--muted) px-4 py-3">
          <span class="font-medium">{model.name}</span>
          <span class="text-sm text-(--muted-foreground)">{model.license}</span>
        </li>
      {/each}
    </ul>
  </section>

  <footer class="mt-auto text-sm text-(--muted-foreground)">
    <a class="underline underline-offset-2" href="https://github.com/kanakkholwal/airlock">GitHub</a>
  </footer>
</main>
