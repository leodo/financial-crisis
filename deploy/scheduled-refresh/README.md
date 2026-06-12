# 免费数据日频刷新定时任务

本目录提供把 `fc-worker refresh latest-free` 固化成系统级定时任务的参考配置。
目标是让免费数据在无人值守时也能按日刷新，并在单源瞬时网络抖动时自动重试，
而不是依赖手工执行 `just refresh-latest`。

刷新命令本身已经内置：

- 阶段级失败隔离：单个源失败不阻塞其它源；
- 失败自动重试：默认每个阶段最多重试 `2` 次（共 3 次尝试），线性退避；
  可用 `--max-retries` / `--retry-backoff-secs` 调整；
- run 级证据落库：成功/失败都会写入 `ingest_runs`，可用 `just refresh-status` 核对。

> 注意：本轮只完成“定时 + 自动重试 + 状态可见”的最小闭环。
> 失败告警推送（邮件 / Webhook / IM）仍是后续工作，当前只把失败记录到
> `ingest_runs` 和刷新日志中。

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

刷新后用以下命令核对免费数据是否真的成功落库：

```bash
fc-worker refresh status   # 或 just refresh-status
```
