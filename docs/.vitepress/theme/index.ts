import DefaultTheme from 'vitepress/theme';
import './style.css';
import DemoFrame from './components/DemoFrame.vue';
import InlineButtons from '../../examples/buttons.vue';

export default { extends: DefaultTheme, enhanceApp({ app }: Parameters<NonNullable<typeof DefaultTheme.enhanceApp>>[0]) { app.component('DemoFrame', DemoFrame); app.component('InlineButtons', InlineButtons); } };
