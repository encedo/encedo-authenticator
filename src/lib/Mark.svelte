<script lang="ts">
  import mark from "../assets/mark.png";
  let { size = 26 }: { size?: number } = $props();
</script>

<!-- The Encedo head, the mark v1 carried, in the rkv.pl green. -->
<img class="mark" src={mark} width={size} height={size} alt="Encedo" />
