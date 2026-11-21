<script lang="ts">
  import { toast } from "svelte-sonner";
  import { chat } from "#lib/chat.svelte.js";
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from "#lib/components/ui/alert-dialog/index.js";

  // One confirmation for every place a chat can be deleted (sidebar row, chat header).
  let {
    open = $bindable(false),
    id,
    title,
  }: { open?: boolean; id: number | null; title: string } = $props();

  async function remove() {
    // Close first: the action's own close does not run when it has a handler.
    open = false;
    if (id === null) return;
    await chat.remove(id);
    toast("Chat deleted");
  }
</script>

<AlertDialog bind:open>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Delete this chat?</AlertDialogTitle>
      <AlertDialogDescription>
        "{title}" and its messages will be removed from this computer. This cannot be undone.
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>Keep</AlertDialogCancel>
      <AlertDialogAction destructive onclick={remove}>Delete</AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
