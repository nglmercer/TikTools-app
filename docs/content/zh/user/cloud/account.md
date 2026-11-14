---
title: "登录 Cloud 账户"
docId: "user.cloud.account"
translationKey: "user.cloud.account"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "02b7718e7fc7b2a53b68ee1c6e19e622608a6fdfc7eac553879c474ca69ea74b"
translationRevision: "dcdf6b0e0597904e86b677257bceda1e83a272fbe55075253c580e829a37db83"
order: 10
---

# 登录 Cloud 账户 {#sign-in-to-your-cloud-account}

## 目标与前置条件 {#goal-and-prerequisites}

将桌面应用连接到已配置的 Cloud 服务，以使用基于账户的目录与协作操作。你需要网络、系统浏览器及可用的操作系统凭据存储。未连接 Cloud 时仍可使用本地编辑；Cloud 会话不能验证 TikTok LIVE 连接。

## 登录并验证身份 {#sign-in-and-verify-identity}

1. 打开 **设置 → 账户**。开始另一轮登录前，先阅读当前状态。
2. 选择 **登录**。桌面程序会启动本地回调接收器，并在系统浏览器中打开已配置的登录页面。
3. 完成服务的登录流程，然后返回 TikTools。如果浏览器未打开，请使用应用为本次尝试显示的登录 URL。该 URL 应保密，不要公开发布。
4. 等待账户状态变为已登录。打开私有目录或团队前，确认显示的名称或邮箱。
5. 状态报告错误时，展开账户诊断信息。使用 **取消** 放弃未完成的尝试，避免同时保留多次活动尝试。
6. 在共用电脑上使用完毕后，选择 **退出登录**，并验证账户状态已改变。

原生宿主管理会话令牌。凭据通过操作系统凭据存储保存，前端仅保留非秘密状态。安全存储不可用时，不会退回明文文件存储。复制数据文件夹不会得到可携带的已登录会话。

## 预期结果与状态 {#expected-result-and-status}

启用 Cloud 身份验证构建的宿主支持原生登录、恢复及退出。已配置账户可在重启时恢复，并向服务重新验证。未配置或不受支持的宿主，不能通过更换 TikTok Cookie 来修复。

服务登录是否可用，以及真实提供商和平台能否通过验收，取决于部署的服务与本机环境。本地验证不能证明所有生产登录路径都正常。

## 恢复方法 {#recovery}

尝试已取消或过期时，关闭旧尝试后重新登录。网络或服务错误请保留诊断代码，待恢复后重试。凭据存储错误需恢复操作系统的安全存储能力；不要将 bearer 令牌粘贴到设置或支持消息中。明确同步操作见[变更](changes.md)，独立访问授权见[协作](collaboration.md)。

## 运行细节 {#operational-details}

退出登录先删除本地凭据并清空账户上下文，再尽力尝试服务器撤销。界面显示已退出只确认本地移除，不证明网络故障时远程令牌已撤销。若服务提供会话管理，可在那里检查。退出或切换已知账户前请导出 Diagnostics，因为该界面历史会清空。TikTok cookie 与远程宿主会话是独立身份。
