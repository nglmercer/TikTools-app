<script setup lang="ts">
import type { ExperienceCopy, Topic } from '../../../experience/types.ts';
defineProps<{ topic: Topic; copy: ExperienceCopy; feed?: boolean }>();
</script>
<template>
  <div class="scene-preview" :class="`scene-preview--${topic}`" aria-hidden="true">
    <div class="scene-preview__bar"><span /><span /><span /><b>TikTools <span>/ {{ copy.topics[topic].title }}</span></b><i>LIVE</i></div>
    <div v-if="feed" class="scene-preview__feed"><div><span>L</span><b>Luna<small>{{ copy.previewLabels.chat }}</small></b><p>{{ copy.previewLabels.message }}</p><time>12:00</time></div><div><span>M</span><b>Milo<small>{{ copy.previewLabels.gift }}</small></b><p>Rose × 1</p><time>12:01</time></div></div>
    <div v-else-if="topic === 'connection'" class="scene-preview__connection">
      <div class="scene-preview__label">{{ copy.previewLabels.creator }}<span class="scene-preview__status">{{ copy.previewLabels.disconnected }}</span></div>
      <div class="scene-preview__input"><span>@</span>demo_creator<span class="scene-preview__cursor" /></div>
      <div class="scene-preview__flow"><span>01</span><b>{{ copy.previewLabels.creator }}</b><span>→</span><span>02</span><b>LIVE</b><span>→</span><span>03</span><b>{{ copy.previewLabels.event }}</b></div>
    </div>
    <div v-else-if="topic === 'automation'" class="scene-preview__automation">
      <div><small>{{ copy.previewLabels.event }}</small><b>tiktok.chat</b><code>!hello</code></div><span>→</span><div><small>{{ copy.previewLabels.action }}</small><b>core.log</b><code>Hello Luna</code></div>
    </div>
    <div v-else-if="topic === 'overlays'" class="scene-preview__overlay"><div class="scene-preview__canvas"><div><span>✦</span><b>Luna</b><small>{{ copy.previewLabels.gift }} · Rose × 1</small></div></div><span class="scene-preview__obs">OBS · 1920 × 1080</span></div>
    <div v-else class="scene-preview__points"><div><span>L</span><b>Luna<small>@luna</small></b><strong>120 <small>pts</small></strong></div><div><span>M</span><b>Milo<small>@milo</small></b><strong>85 <small>pts</small></strong></div><div class="scene-preview__meter"><span /></div></div>
  </div>
</template>
