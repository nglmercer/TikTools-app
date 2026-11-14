<script setup lang="ts">
import { computed, ref } from 'vue';
import { useData } from 'vitepress';
import { documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import snapshot from '../../../experience/releases.json';
const { lang } = useData();
const locale = computed(() => documentLocale(lang.value));
const t = computed(() => experienceCopy[locale.value].releases);
const system = ref('all');
const date = (value: string) => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeZone: 'UTC' }).format(new Date(value));
const assets = (release: typeof snapshot.releases[number]) => release.assets.filter((asset) => !/SHA256|\.json$|\.sig$|\.txt$/i.test(asset.name) && (system.value === 'all' || asset.name.toLowerCase().includes(system.value)));
</script>
<template>
  <section class="release-list" :aria-label="t.published">
    <div class="release-list__toolbar">
      <span>{{ t.downloads }}</span>
      <div class="release-list__filters" role="group" :aria-label="t.downloads">
        <button v-for="option in ['all', 'windows', 'linux']" :key="option" type="button" :aria-pressed="system === option" @click="system = option">{{ option === 'all' ? t.all : option === 'windows' ? 'Windows' : 'Linux' }}</button>
      </div>
    </div>
    <article v-for="release in snapshot.releases" :key="release.tag" class="release-card">
      <header><div><span class="release-card__channel" :class="{ 'release-card__channel--pre': release.prerelease }">{{ release.prerelease ? t.prerelease : t.stable }}</span><h2>{{ release.name }}</h2></div><time :datetime="release.publishedAt">{{ date(release.publishedAt) }}</time></header>
      <details v-if="release.highlights.length" class="release-card__notes" :open="release.tag === snapshot.releases[0]?.tag"><summary>{{ t.highlights }}</summary><ul lang="en"><li v-for="line in release.highlights" :key="line">{{ line }}</li></ul></details>
      <div class="release-card__assets">
        <a v-for="asset in assets(release)" :key="asset.name" :href="asset.url"><span aria-hidden="true">↓</span><span>{{ asset.name }}<small>{{ (asset.size / 1048576).toFixed(1) }} MB</small></span></a>
        <p v-if="!assets(release).length">{{ t.noDownloads }}</p>
      </div>
      <footer><a :href="release.url">{{ t.notes }} ↗︎</a><a :href="`https://github.com/nglmercer/TikTools-app/tree/${encodeURIComponent(release.tag)}`">{{ t.source }}</a><a v-for="asset in release.assets.filter(asset => /SHA256SUMS/i.test(asset.name))" :key="asset.name" :href="asset.url">{{ t.checksums }}</a></footer>
    </article>
    <p class="release-list__snapshot">{{ t.snapshot }}: <time :datetime="snapshot.checkedAt">{{ date(snapshot.checkedAt) }}</time>. <a href="https://github.com/nglmercer/TikTools-app/releases">{{ t.refresh }} ↗︎</a></p>
  </section>
</template>
