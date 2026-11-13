export default {
  controls: { next: "下一步", previous: "上一步", done: "完成指南", close: "关闭指南", progress: "第 {{current}} 步，共 {{total}} 步" },
  connection: {
    form: { title: "选择主播", description: "输入主播用户名并审查连接状态。指南绝不会进行连接，也不会读取你的会话 Cookie。" },
    explorer: { title: "探索直播间", description: "请自行刷新或选择直播间。演示中的成功连接是模拟的；原生访问取决于主机和 TikTok。" },
    plugins: { title: "检查集成", description: "启用效果前，请审查每个已安装连接及其健康状态。打开本指南不会联系插件服务。" },
  },
  automation: {
    profiles: { title: "选择工作配置方案", description: "为要审查的规则保留一个配置方案。切换或导入配置方案需要在应用中明确操作。" },
    library: { title: "审查事件和操作", description: "事件将触发器、筛选器和冷却时间与可复用操作关联起来。新效果应保持禁用，直到其定义经过审查。" },
    templates: { title: "从模板开始", description: "打开“规则模板”以预览可复用配置。导入和启用分别需要审查；本指南不会执行这两项操作。" },
  },
  plugin: {
    catalog: { title: "已安装与探索", description: "审查已安装的包，或浏览兼容的目录条目。原生插件和进程插件保留其用户的操作系统权限。" },
    search: { title: "查找包", description: "搜索并检查发布者、版本和要求。目录为空和后端不可用是两种不同状态。" },
    install: { title: "审查安装", description: "安装有独立的签名和权限审查流程。指南不会安装、批准新权限或运行测试操作。" },
  },
  overlay: {
    gateway: { title: "检查网关状态", description: "实时输出需要 event-gateway 插件和组件凭据。本地预览不能证明已连接 OBS。" },
    preview: { title: "本地预览", description: "预览使用相同的组件渲染器和示例事件，无需真实直播会话，也不会向 OBS 发送提醒。" },
    design: { title: "审查设计", description: "明确编辑并保存后，再复制更新的源 URL 供 OBS 使用。“叠加层更改”仍是尚未合并的后续工作。" },
  },
  points: {
    rules: { title: "审查积分获取规则", description: "更改前请检查已配置的值。直播会话已连接或繁忙时，部分控件会锁定。" },
    leaderboard: { title: "检查已提交的积分", description: "搜索或排序观众。刷新显示不会调整或重置积分，也不会重放直播事件。" },
  },
  changes: {
    library: { title: "选择记录", description: "事件、操作和变量有各自独立的审查边界。审查其他状态前，必须保留编辑器中未保存的候选内容。" },
    review: { title: "审查更改", description: "“自动化更改”仅在本地进行。“已审查变量”使用原生修订版检查；本次审查不会上传原始变量值。" },
    profiles: { title: "明确执行恢复", description: "未知确认结果必须通过其原始日志处理。关闭本指南既不会完成任务，也不会重试更改操作。" },
  },
  diagnostics: {
    log: { title: "检查有限历史", description: "按来源、级别或文本筛选，并检查重复条目。分享前请审查经过敏感信息遮蔽的导出内容；历史仅限当前会话。" },
    back: { title: "审查证据并恢复", description: "复制或导出已审查的证据，再使用原始流程处理冲突、会话过期或不确定的命令。清空日志不会修复错误。" },
  },
} as const;
