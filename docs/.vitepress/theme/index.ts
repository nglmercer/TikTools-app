import DefaultTheme from 'vitepress/theme';
import './style.css';
import './experience.css';
import DemoFrame from './components/DemoFrame.vue';
import DocsHome from './components/DocsHome.vue';
import InlineButtons from '../../examples/buttons.vue';
import DocSteps from './components/DocSteps.vue';
import DocCallout from './components/DocCallout.vue';
import ReleaseList from './components/ReleaseList.vue';
import DocRequirements from './components/DocRequirements.vue';
import DocTabs from './components/DocTabs.vue';

export default {
  extends: DefaultTheme,
  enhanceApp({ app }: Parameters<NonNullable<typeof DefaultTheme.enhanceApp>>[0]) {
    app.component('DemoFrame', DemoFrame);
    app.component('DocsHome', DocsHome);
    app.component('InlineButtons', InlineButtons);
    app.component('DocSteps', DocSteps);
    app.component('DocCallout', DocCallout);
    app.component('ReleaseList', ReleaseList);
    app.component('DocRequirements', DocRequirements);
    app.component('DocTabs', DocTabs);
  },
};
