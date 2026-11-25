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
  feed: {
    view: { title: "选择 Feed 视图", description: "实时 Feed 以卡片显示聊天、礼物和关注；切换到“列表”可查看紧凑日志。" },
    filters: { title: "筛选活动", description: "按类型筛选或按观众和消息搜索。筛选同样适用于 Feed 卡片。" },
    stage: { title: "阅读实时墙", description: "重要消息会保留。点赞和加入会短暂显示为互动提示，然后淡出。" },
    controls: { title: "暂停、清空与查看", description: "暂停自动滚动以便阅读，清空已保存的历史记录，或打开检查器查看详情。" },
  },
  backup: {
    plan: { title: "了解额度", description: "每个套餐都有各自的存储和备份额度。这些进度条显示你已使用的量。" },
    tools: { title: "云端或此电脑", description: "在保存到云端账户与处理此电脑上的文件之间切换：导出你的数据，或上传已有的文件。" },
    create: { title: "保存备份", description: "为备份命名并保存到账户。它包含规则、配置、设置和云端文件，不含密钥或 Cookie。" },
    list: { title: "下载、恢复或删除", description: "每个已保存的备份都可以下载、在登录此账户的任何设备上恢复，或删除。恢复前会先显示将发生的变更。" },
  },
  analytics: {
    range: { title: "选择日期范围", description: "选择预设或自定义日期。摘要会针对当前创作者重新加载；更改范围永远不会修改已保存的事件。" },
    metrics: { title: "选择指标", description: "卡片比较聊天、礼物、点赞和进入。所选指标只驱动活动图表，不会修改分析数据。" },
    activity: { title: "阅读活动图表", description: "使用图表找出活跃的日期和时段。空白时段可能表示没有 LIVE 或没有匹配事件。" },
    facts: { title: "查看关键数据", description: "总数和会话次数提供紧凑概览，便于之后比较单个观众。" },
    contributors: { title: "搜索贡献者", description: "排序或搜索表格，找出最活跃的观众。这是只读排行榜。" },
  },
  variables: {
    library: { title: "打开 Variables", description: "变量与自动化一起管理。打开 Variables 标签页，检查事件和操作共享的值。" },
    values: { title: "检查值和密钥", description: "本地值可以标记为密钥。编辑器永远不会显示已保存的密钥，保存始终是明确操作。" },
    review: { title: "同步前先比较", description: "使用 Changes 比较本地和 Cloud 修订。先检查差异；向导不会上传原始密钥值。" },
  },
  settings: {
    sections: { title: "选择设置分区", description: "General 包含应用偏好设置。Account 包含 Cloud 个人资料、套餐和备份工具。" },
    language: { title: "更改语言", description: "语言会立即应用到界面。它不会更改 LIVE 数据或自动化行为。" },
    review: { title: "选择审核模式", description: "审核模式控制敏感同步流程需要多少确认。更安全的模式需要更明确的审核。" },
    updater: { title: "手动检查更新", description: "使用更新器卡片查看通道并检查更新。如果无法更新，请按照文档中的恢复路径下载版本。" },
  },
  overlayObs: {
    gateway: { title: "从网关开始", description: "OBS 小部件通过本地网关接收事件。复制任何源 URL 前，先确认网关正在运行。" },
    widget: { title: "选择小部件", description: "选择要放入 OBS 的 follow、gift、chat、likes 或 ranking 源。" },
    preview: { title: "用预览测试", description: "重放示例事件并在本地调整设计。预览不能证明 OBS 已连接。" },
    url: { title: "复制 OBS URL", description: "只有网关就绪后才复制源 URL，然后按所需大小粘贴到 OBS Browser Source。" },
    design: { title: "调整动画和外观", description: "设计编辑器现在包含动画样式、时长、缓动和交错。复制更新后的 URL 前先保存。" },
  },
  cloudOverlays: {
    open: { title: "打开 Cloud 库", description: "Cloud 悬浮层可用时，通过 Cloud 库按钮浏览共享的悬浮层文档。" },
    widgets: { title: "选择本地目标", description: "应用任何内容前，先选择哪个本地小部件应接收所选 Cloud 设计。" },
    preview: { title: "发布前预览", description: "使用本地预览比较所选设计。发布或覆盖 Cloud 副本仍然是明确操作。" },
    url: { title: "复制正确的源", description: "本地网关 URL 和 Cloud 悬浮层 URL 用于不同的恢复场景。确认你的 OBS 场景应使用哪一种。" },
  },
  automationAdvanced: {
    library: { title: "有意识地打开事件", description: "高级行为位于事件内部。实验前先选择或创建一个已停用的事件。" },
    editor: { title: "检查筛选、冷却和顺序", description: "在编辑器中，条件、备用项、冷却范围、随机模式和延迟会精确决定操作何时运行。" },
    templates: { title: "以模板作为起点", description: "模板可以包含更安全的默认值。导入仍需审核，并且不会单独启用规则。" },
    runs: { title: "用测试和历史验证", description: "启用事件前，使用测试运行和历史面板确认时机、跳过的操作和冷却行为。" },
  },
  rankings: {
    tabs: { title: "排行榜或竞赛", description: "排行榜累计分数。竞赛在限定活动期间比较所选排行榜。" },
    list: { title: "选择或创建排行榜", description: "列表显示指标、模式和状态。只有决定哪种观众行为计分后才创建新排行榜。" },
    detail: { title: "检查实时记分板", description: "详情视图显示分数、暂停状态和完整性。暂停、恢复和重置都是明确操作。" },
    competitions: { title: "配置竞赛", description: "竞赛需要兼容的排行榜和清晰边界。它们仍是实验性功能，直播使用前请验证悬浮层。" },
  },
  pluginSetup: {
    catalog: { title: "打开已安装插件", description: "配置这台机器上已信任的插件时，从 Installed 而不是 Explore 开始。" },
    details: { title: "检查权限和状态", description: "详情头部显示启用状态、版本以及影响运行时的操作。启用前先检查权限。" },
    settings: { title: "配置设置", description: "Settings 标签页包含插件专属值。如果插件要求，测试前先保存。" },
    connection: { title: "连接并测试", description: "Connection 标签页管理外部服务凭据，并在插件提供时显示测试操作。" },
  },
  connectionRecovery: {
    form: { title: "重新检查创作者和状态", description: "从准确的创作者句柄和当前连接状态开始。离线、认证和房间不可用需要不同的恢复方式。" },
    explorer: { title: "验证 LIVE 房间", description: "刷新房间列表并确认房间可访问。找不到的房间可能离线、受限，或只是不向访客开放。" },
    plugins: { title: "重连后检查集成", description: "连接集成可能滞后于重连。在判断自动化或悬浮层失败前，先检查其健康状态。" },
  },
  feedInspector: {
    controls: { title: "打开检查器", description: "使用动态工具栏中的检查器控件。它不会暂停 LIVE，也不会更改已保存事件。" },
    panel: { title: "理解面板", description: "检查器包含观众、事件详情和资料标签页。内容会跟随你选择的事件或观众。" },
    viewers: { title: "比较活跃观众", description: "在顶级观众和点赞之间切换，查看谁在推动当前会话的活动。" },
    details: { title: "检查单个事件", description: "事件详情包括次数、积分、时间和消息证据，可用于诊断自动化行为。" },
    profile: { title: "打开观众资料", description: "资料标签页汇总观众及其最近消息，而不暴露账户私人数据。" },
  },
  backupRestore: {
    tools: { title: "选择恢复来源", description: "Cloud 备份和本地文件是分开的恢复路径。检查将发生的变化前，先选择来源。" },
    list: { title: "谨慎恢复", description: "备份行可以下载、恢复或删除。恢复前始终显示受影响数据的审核内容。" },
    plan: { title: "验证结果", description: "恢复后检查套餐、配置文件、设置和关键自动化。成功恢复不会重新验证所有外部凭据。" },
  },
} as const;
