<script lang="ts">
  import type { Snippet } from "svelte";
  let { icon, children, class: className = "", onclick, title, "aria-label": ariaLabel }: {
    icon?: Snippet;
    children?: Snippet;
    class?: string;
    onclick?: () => void;
    title?: string;
    "aria-label"?: string;
  } = $props();
</script>

<button class={className} onclick={onclick} title={title} aria-label={ariaLabel}>
  {#if icon} {@render icon()} {/if}
  {#if children} {@render children()} {/if}
</button>
