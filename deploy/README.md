# 部署指南

## 目录结构

```
/opt/financial-crisis/
├── current -> releases/v20260612_123456/  # symlink
├── releases/
│   ├── v20260612_123456/                  # 每次更新创建新目录
│   │   ├── bin/fc-api
│   │   ├── bin/fc-worker
│   │   ├── web/dist/
│   │   ├── config/
│   │   ├── COMMIT                         # commit hash
│   │   └── COMMIT_MSG                     # commit message
│   └── v20260613_.../
├── data/fc-local.sqlite                   # 持久数据（不参与更新）
├── logs/
│   ├── api.log
│   ├── refresh.log
│   ├── update.log
│   ├── deploy-check-*.md
│   └── daily-health-*.md
├── deploy/                                # 服务配置
│   ├── fc-api.env
│   ├── fc-api.service -> /etc/systemd/system/fc-api.service
│   ├── fc-refresh.timer
│   ├── operational-check.sh               # 部署/回滚/刷新后置验收
│   └── smoke-check.sh                     # 当前 release/API/systemd 快速冒烟检查
├── update.sh                              # 一键更新
└── rollback.sh                            # 一键回滚
```

## 首次部署

```bash
# 1. 在服务器上 clone 仓库
git clone <repo-url> /opt/financial-crisis-src

# 2. 检查系统依赖
#    需要: cargo (Rust), node/npm, systemd
which cargo node npm systemctl

# 3. 运行引导脚本（会自动完成全部初始化）
sudo bash /opt/financial-crisis-src/deploy/bootstrap.sh
```

## 日常更新

每次要部署新版本时：

```bash
# 登录服务器
ssh root@45.32.75.106

# 进入源码目录
cd /opt/financial-crisis-src

# 拉取最新代码
git pull

# 部署（编译 + 构建 + 切换版本 + 重启服务 + 自动验收）
sudo bash deploy/update.sh

# 如需手动补查
sudo /opt/financial-crisis/deploy/operational-check.sh --mode deploy
sudo /opt/financial-crisis/deploy/smoke-check.sh \
  --expected-commit "$(cat /opt/financial-crisis/current/COMMIT)" \
  --public-url http://45.32.75.106
```

**更新过程会自动：**
1. `git pull` 拉取最新代码
2. `cargo build --release` 编译 Rust
3. `npm run build` 构建前端
4. 创建 `releases/vYYYYMMDD_HHMMSS/` 目录
5. 切换 `current` symlink
6. 重启 `fc-api` 服务
7. 运行 `/opt/financial-crisis/deploy/operational-check.sh --mode deploy`，生成部署验收报告
8. 运行 `/opt/financial-crisis/deploy/smoke-check.sh`，检查当前 release、API、定时器、数据新鲜度和生产源状态
9. 修复 `/opt/financial-crisis/data`、`logs` 的 `fc-service` 写权限，并同步 systemd service/timer 配置
10. 清理旧版本（保留最近 3 个）

如果是首次部署且 `fc-api` service 尚未安装，`update.sh` 会先跳过本阶段验收；
`bootstrap.sh` 会在安装并启用 systemd 服务后运行 `--mode bootstrap` 验收。

## 回滚

```bash
# 恢复到上一个版本
sudo bash deploy/rollback.sh

# 恢复到指定版本
sudo bash deploy/rollback.sh v20260612_123456

# 查看可用版本
ls /opt/financial-crisis/releases/
```

回滚脚本会在切换 `current` 并重启 API 后自动运行
`/opt/financial-crisis/deploy/operational-check.sh --mode rollback` 和
`/opt/financial-crisis/deploy/smoke-check.sh`。

## 系统服务

```bash
# API 服务
systemctl status fc-api           # 状态
journalctl -u fc-api -n 50       # 最近 50 行日志
systemctl restart fc-api          # 重启

# 数据刷新（工作日 06:30 自动执行）
systemctl status fc-refresh.timer       # 定时器状态
systemctl list-timers fc-refresh.timer  # 下次执行时间
systemctl start fc-refresh.service      # 手动触发一次刷新
```

`fc-refresh.service` 刷新成功后会自动执行
`/opt/financial-crisis/deploy/operational-check.sh --mode refresh`，
并在 `/opt/financial-crisis/logs/` 写入 `deploy-check-refresh-*.md`
、`daily-health-refresh-*.md` 和 `risk-threshold-refresh-*.md`。

## 验证部署

```bash
# 1. 一键验收 API、assessment、sources、关键日期和生产源降级
sudo /opt/financial-crisis/deploy/operational-check.sh --mode deploy

# 1b. 快速检查当前 release、systemd、SQLite 数据模式、关键指标新鲜度和公网首页
sudo /opt/financial-crisis/deploy/smoke-check.sh \
  --expected-commit "$(cat /opt/financial-crisis/current/COMMIT)" \
  --public-url http://45.32.75.106

# 2. 如需排查数据源，再看底层刷新证据
/opt/financial-crisis/current/bin/fc-worker refresh status

# 3. 如需单独留存每日健康摘要
FC_API_BASE_URL=http://127.0.0.1:18080 node scripts/daily-health-report.mjs \
  --output /opt/financial-crisis/logs/daily-health-$(date +%Y%m%d-%H%M%S).md \
  --fail-on-issues

# 4. 当前运行版本
readlink /opt/financial-crisis/current
cat /opt/financial-crisis/current/COMMIT_MSG

# 5. 查看更新历史
tail -30 /opt/financial-crisis/logs/update.log
```

## 主动告警

默认不发送外部告警，只写入日志和 Markdown 报告。需要主动推送时，在
`/opt/financial-crisis/deploy/fc-api.env` 中配置至少一个 webhook：

```bash
# 通用 JSON webhook
FC_ALERT_WEBHOOK_URL=https://example.com/financial-crisis-alert
# FC_ALERT_WEBHOOK_BEARER_TOKEN=...

# 或常见 IM webhook，按实际平台选择一个
# FC_ALERT_SLACK_WEBHOOK_URL=...
# FC_ALERT_FEISHU_WEBHOOK_URL=...
# FC_ALERT_DINGTALK_WEBHOOK_URL=...
```

部署、回滚、刷新后的 `operational-check.sh` 会在 `deploy-check` 或
`daily-health-report` 返回非 0 时主动推送异常摘要和报告摘录。默认只提醒；
不会触发任何自动交易或自动仓位调整。

上线前可先运行 `just operational-alert-dry-run` 预览 payload；该命令不会发送外部消息。

刷新后的业务层阈值提醒由 `scripts/risk-threshold-alert.mjs` 生成，默认阈值为：

```bash
FC_RISK_ALERT_OVERALL_SCORE=55
FC_RISK_ALERT_TRIGGER_SCORE=60
FC_RISK_ALERT_MIN_POSTURE=prepare
FC_RISK_ALERT_MAX_SOURCE_ISSUES=0
FC_RISK_ALERT_ON_REFERENCE_ONLY=0
```

这类提醒只说明“需要人工复核”，不会让系统自动交易；如需预览可运行
`just risk-threshold-alert-dry-run`。

## Nginx 反向代理（可选）

如需要通过域名访问，在 `/etc/nginx/sites-available/financial-crisis`:

```nginx
server {
    listen 80;
    server_name crisis.example.com;

    location / {
        proxy_pass http://127.0.0.1:18080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

## 关键原则

1. **数据不动**：`data/` 和 `logs/` 存储在 release 目录之外，更新不丢失
2. **版本隔离**：每次更新创建独立 release 目录，不会覆盖旧文件
3. **自动清理**：`update.sh` 自动保留最近 3 个版本，删除更旧的
4. **更新可回滚**：旧版本目录还在时，一秒切回
5. **更新日志**：每次部署记录 commit、时间、操作结果到 `logs/update.log`
