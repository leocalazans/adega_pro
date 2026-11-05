<script lang="ts">
  export let size = 18;
  export let color = "currentColor";
</script>

<svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} stroke-width="2" aria-hidden="true">
  <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
  <path d="M7 11V7a5 5 0 0 1 10 0v4" />
  <circle cx="12" cy="16" r="1" fill={color} />
</svg>
