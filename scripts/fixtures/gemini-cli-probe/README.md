# Gemini CLI 探测依赖锁

关联 REQ-20260908-012；[行为证据与运行命令](../../../docs/development/2026-09-08-gemini-cli-capability.md)。这里只保存公开 npm 依赖清单与下载完整性元信息，不保存认证、合成账户、原始终端或运行日志。两个 lockfile 是报告实际运行时的原文件；虽然 package.json 记录 semver 范围，`npm ci` 严格按锁文件恢复固定版本。

在仓库根目录执行以下步骤，只写入专用临时目录，不全局安装，也不运行 npm 生命周期脚本：

```sh
mkdir -p /private/tmp/req012-gemini-probe /private/tmp/req012-gemini-startup
cp scripts/fixtures/gemini-cli-probe/capability/package*.json /private/tmp/req012-gemini-probe/
npm ci --prefix /private/tmp/req012-gemini-probe --ignore-scripts --no-audit --no-fund
cp scripts/fixtures/gemini-cli-probe/startup/package*.json /private/tmp/req012-gemini-startup/
npm ci --prefix /private/tmp/req012-gemini-startup --ignore-scripts --no-audit --no-fund
```

下载步骤需要 npm 公网访问；后续探测严格禁止真实后端请求。Task1 另需开发日志注明的固定上游源码及 `--experimental-vm-modules`；Task4a 运行器需要测试用 Node 26.7.0 和 macOS 系统 Python PTY。这些不是产品依赖。`npm ci` 会替换专用目录内 node_modules，不能把 prefix 换成用户或产品目录。

| 锁文件 | SHA-256 |
| --- | --- |
| capability/package-lock.json | cbace9eaacd30cc93bd35191839e9d1c0dcc66a5bdcde647c901b1d50641caa4 |
| startup/package-lock.json | 8257f064a811bc7ba6471ff1fd824de50b292f87aabe91c7f170a9dafd3ca5d9 |
