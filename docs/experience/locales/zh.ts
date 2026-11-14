import type { ExperienceCopy } from '../types.ts';

const copy: ExperienceCopy = {
  interactive: '交互式示例', fictional: '虚构数据', tryIt: '试用控件', reset: '重置示例', openFull: '打开完整演示', close: '关闭示例',
  preview: '预览', loading: '正在加载真实界面…', task: '练习任务', success: '检查结果',
  topicsTitle: '边操作边学习', topicsDescription: '每项流程都提供真实界面、简短任务和可检查的结果。选择接下来要学习的内容。', readGuide: '阅读指南',
  topics: {
    connection: { title: '连接你的直播', description: '选择主播，查看传入的观众事件。', task: '连接 demo_creator，再查看一条传入事件。', result: '连接状态变为已连接，虚构事件出现在实时动态中。' },
    automation: { title: '将事件转为操作', description: '创建聊天规则，检查执行结果。', task: '查看示例操作，运行测试，再检查运行结果。', result: '自动化历史中显示已完成的运行及其渲染结果。' },
    overlays: { title: '为直播设计叠加层', description: '预览小组件，准备 OBS 浏览器源。', task: '选择小组件，调整外观，再重播预览。', result: '预览反映你的设计。向真实 OBS 输出仍需运行中的网关。' },
    points: { title: '奖励你的观众', description: '设置积分规则，了解观众余额。', task: '查看积分规则，对比事件发生前后的观众余额。', result: '示例余额会更新。演示采用固定的示例奖励；真实倍率需在直播中验证。' },
  },
  otherTasks: {
    feed: { task: '推进示例事件，选择一条动态记录，再打开观众资料。', result: '无需连接 TikTok，即可查看虚构事件详情及观众的已跟踪活动。' },
    analytics: { task: '查看当前统计，推进一个示例事件，再对比更新后的活动。', result: '统计会反映模拟事件，不代表真实直播的数据。' },
    plugins: { task: '查看示例插件，审查权限，再试用模拟的安装控件。', result: '插件状态只在演示中变化，不会联系软件包来源、外部服务或音频设备。' },
    cloud: { task: '比较本地 greeting 变量与 Cloud 示例，再选择要应用的已审查更改。', result: '虚构的差异在本地解决，不会修改 Cloud 账户或远程数据。' },
    collaboration: { task: '审查虚构的设备请求，仅批准你打算授予的有限访问权限。', result: '示例身份会显示已审查的权限，不会有外部设备加入本演示。' },
    settings: { task: '修改一项本地偏好，检查效果，再重置演示。', result: '该偏好仅应用于当前演示实例；重置后恢复初始状态。' },
    diagnostics: { task: '选择失败记录，查看详情，再重试模拟操作。', result: '无需联系外部服务，即可追踪示例故障及重试结果。' },
  },
  previewLabels: { creator: '主播', event: '观众事件', action: '自动化', output: '直播画面', chat: '聊天消息', gift: '收到礼物', connected: '已连接', disconnected: '未连接', message: '你好，TikTools！' },
  connectionStates: { label: '连接示例', normal: '连接成功', offline: '主播未开播', auth: '需要会话', offlineHelp: '主播尚未开播。请确认直播已经开始，再重试连接。', authHelp: '此直播间需要授权的 TikTok 会话。在桌面应用中，将新的会话 Cookie 填入遮蔽内容的字段。请勿将凭据粘贴到本演示中。' },
  releases: { stable: '稳定版', prerelease: '预发行版', notes: '发行说明', source: '源码', downloads: '下载', noDownloads: '此快照中没有适用于此系统的已发布下载。', checksums: 'SHA-256 校验和', published: '已发布', snapshot: '发行快照验证日期', refresh: '在 GitHub 查看最新发行版', all: '全部', highlights: '主要变化', unavailable: '发行数据暂不可用。请在 GitHub 查看已发布发行版。' },
};
export default copy;
