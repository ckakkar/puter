<script lang="ts">
  /** A labelled range input whose value can also be typed for precision. */
  let {
    id,
    label,
    value,
    min,
    max,
    step,
    unit = '',
    digits = 2,
    /** Bounds for typed values; defaults to the slider's range. */
    inputMin = min,
    inputMax = max,
    onchange,
  }: {
    id: string;
    label: string;
    value: number;
    min: number;
    max: number;
    step: number;
    unit?: string;
    digits?: number;
    inputMin?: number;
    inputMax?: number;
    onchange: (value: number) => void;
  } = $props();
  let field: HTMLInputElement;
  const format = (v: number) => (Number.isFinite(v) ? v.toFixed(digits) : '');
  // Live values refresh constantly; never overwrite what someone is typing.
  $effect(() => {
    const text = format(value);
    if (field && document.activeElement !== field) field.value = text;
  });
  function commit() {
    const typed = Number(field.value);
    if (field.value.trim() === '' || !Number.isFinite(typed)) field.value = format(value);
    else onchange(Math.min(inputMax, Math.max(inputMin, typed)));
  }
</script>

<div class="slider-label">
  <label for={id}>{label}</label>
  <span class="slider-value"
    ><input
      bind:this={field}
      class="slider-number"
      type="number"
      inputmode="decimal"
      min={inputMin}
      max={inputMax}
      {step}
      aria-label="{label} value"
      onchange={commit}
      onkeydown={(e) => {
        if (e.key === 'Enter') field.blur();
        if (e.key === 'Escape') {
          field.value = format(value);
          field.blur();
        }
      }}
    />{#if unit}<small>{unit}</small>{/if}</span
  >
</div>
<input
  {id}
  type="range"
  {min}
  {max}
  {step}
  {value}
  aria-valuetext="{format(value)} {unit}"
  oninput={(e) => onchange(+e.currentTarget.value)}
/>
