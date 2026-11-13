<template>
  <div class="doc-steps"><slot /></div>
</template>

<style>
.doc-steps > ol { list-style:none; padding:0; counter-reset:guide-step; }
.doc-steps > ol > li { position:relative; counter-increment:guide-step; padding:5px 0 20px 48px; margin:0; border-left:1px solid var(--vp-c-divider); margin-left:15px; }
.doc-steps > ol > li:last-child { border-left-color:transparent; padding-bottom:4px; }
.doc-steps > ol > li::before { content:counter(guide-step, decimal-leading-zero); position:absolute; left:-16px; top:2px; display:grid; place-items:center; width:32px; height:32px; background:var(--vp-c-bg); border:1px solid var(--vp-c-divider); border-radius:50%; color:var(--vp-c-brand-1); font:600 12px var(--vp-font-family-mono); }
.doc-steps > ol > li p:first-child { margin-top:0; }
</style>
