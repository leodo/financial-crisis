# 免费数据日频刷新定时任务

本目录提供把 `fc-worker refresh latest-free` 固化成系统级定时任务的参考配置。
目标是让免费数据在无人值守时也能按日刷新，并在单源瞬时网络抖动时自动重试，
而不是依赖手工执行 `just refresh-latest`。

刷新命令本身已经内置：

- 阶段级失败隔离：单个源失败不阻塞其它源；
- 失败自动重试：默认每个阶段最多重试 `2` 次（共 3 次尝试），线性退避；
  可用 `--max-retries` / `--retry-backoff-secs` 调整；
- run 级证据落库：成功/失败都会写入 `ingest_runs`，可用 `just refresh-status` 核对。
- 自动后置验收：systemd service 的 `ExecStartPost` 会运行
  `/opt/financial-crisis/deploy/operational-check.sh --mode refresh`，
  生成部署验收、每日健康和业务阈值 Markdown 报告。

> 注意：本轮只完成“定时 + 自动重试 + 自动后置验收 + 状态可见”的最小闭环。
> Webhook / IM 告警已预留并接入后置验收，但默认不启用；当前未配置 webhook
> 时会把失败记录到 `ingest_runs`、刷新日志和后置验收报告中。

## Linux：systemd timer（推荐）

把 `financial-crisis-refresh.service` 和 `.timer` 放到 `/etc/systemd/system/`，
按实际路径修改 `WorkingDirectory`、`ExecStart` 与 `Environment`，然后：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now financial-crisis-refresh.timer
systemctl list-timers financial-crisis-refresh.timer   # 确认下次触发时间
journalctl -u financial-crisis-refresh.service -n 100   # 查看刷新日志
```

默认在工作日 `06:30`（本地时区）刷新一次，并带 `RandomizedDelaySec` 抖动，
避免和数据源整点高峰对齐。

## Linux：crontab（备选）

如果环境没有 systemd，可以用 cron。参考 `refresh.cron`：

```cron
30 6 * * 1-5 cd /opt/financial-crisis && /usr/local/bin/fc-worker refresh latest-free --mvp-key-only --fast-lookback-days 14 --fred-chunk-days 15 >> /var/log/financial-crisis/refresh.log 2>&1
```

## Windows：计划任务

```powershell
$action = New-ScheduledTaskAction -Execute "fc-worker.exe" `
  -Argument "refresh latest-free --mvp-key-only --fast-lookback-days 14 --fred-chunk-days 15" `
  -WorkingDirectory "D:\project\develop\financial-crisis"
$trigger = New-ScheduledTaskTrigger -Daily -At 6:30AM
Register-ScheduledTask -TaskName "financial-crisis-refresh" -Action $action -Trigger $trigger
```

## 校验

systemd service 已经在刷新成功后自动运行后置验收；如需手动补查：

```bash
fc-worker refresh status   # 或 just refresh-status
sudo /opt/financial-crisis/deploy/operational-check.sh --mode refresh
```

`operational-check.sh` 内部复用 `deploy-check --fail-on-issues` 和
`daily-health-report --fail-on-issues`，提供稳定退出码；生产源降级或
runtime stale warning 出现时会让任务返回非 0。如已配置
`FC_ALERT_WEBHOOK_URL`、`FC_ALERT_SLACK_WEBHOOK_URL`、`FC_ALERT_FEISHU_WEBHOOK_URL`
或 `FC_ALERT_DINGTALK_WEBHOOK_URL`，异常会主动推送；默认只提醒，不自动交易。

业务层阈值提醒由 `risk-threshold-alert.mjs` 生成，默认看总风险分、触发压力分、
MVP/动作档位、生产源降级和 runtime stale warning。阈值可用
`FC_RISK_ALERT_OVERALL_SCORE`、`FC_RISK_ALERT_TRIGGER_SCORE`、
`FC_RISK_ALERT_MIN_POSTURE`、`FC_RISK_ALERT_MAX_SOURCE_ISSUES` 配置；它只发提醒，
不改变仓位、不触发自动交易。
