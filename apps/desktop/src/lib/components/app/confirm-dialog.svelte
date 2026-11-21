<script lang="ts">
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

  // One confirmation for destructive actions that cannot be undone.
  let {
    open = $bindable(false),
    title,
    description,
    confirmLabel = "Delete",
    cancelLabel = "Keep",
    onconfirm,
  }: {
    open?: boolean;
    title: string;
    description: string;
    confirmLabel?: string;
    cancelLabel?: string;
    onconfirm: () => void | Promise<void>;
  } = $props();
</script>

<AlertDialog bind:open>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>{title}</AlertDialogTitle>
      <AlertDialogDescription>{description}</AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>{cancelLabel}</AlertDialogCancel>
      <AlertDialogAction
        destructive
        onclick={() => {
          // Close first: the action's own close does not run when it has a handler.
          open = false;
          void onconfirm();
        }}
      >
        {confirmLabel}
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
