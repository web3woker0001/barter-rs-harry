# Crypto Monitor - 实时加密货币监控系统

基于 Barter-RS 和 Fluvio 构建的高性能加密货币实时监控系统，支持多交易所数据采集、异常检测、自动交易和多渠道通知。

## 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        前端展示层                                 │
│  Web UI / Mobile App / API Client / Dashboard                   │
└─────────────────┬────────────────────────────┬─────────────────┘
                  │         WebSocket          │ REST API
┌─────────────────▼────────────────────────────▼─────────────────┐
│                        API 服务层                                │
│  WebSocket Server (实时推送) | REST API Server (查询/控制)        │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────▼───────────────────────────────┐
│                      核心监控引擎                                 │
│  异常检测器 | 告警管理器 | 自动交易引擎                            │
│  Fluvio Event Stream Bus (分布式消息队列)                        │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────▼───────────────────────────────┐
│                      数据采集层                                   │
│  Barter-RS Engine (多交易所统一接口)                              │
│  Binance | Coinbase | OKX | Bybit | Gate.io | Kraken ...       │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────▼───────────────────────────────┐
│                      通知服务层                                   │
│  Telegram | WeChat | Email | SMS                                │
└─────────────────────────────────────────────────────────────────┘
```

## 核心功能

### 1. 多交易所实时数据采集
- 支持 8+ 主流交易所（Binance, Coinbase, OKX, Bybit 等）
- 实时 WebSocket 数据流
- 统一数据格式标准化
- 自动重连和错误恢复

### 2. 智能异常检测
- **交易量异常检测**：基于 Z-Score 和移动平均
- **价格突变监控**：百分比变化和波动率分析
- **市场深度异常**：买卖盘失衡检测
- **大额交易追踪**：巨鲸活动监控
- 可配置的检测阈值和窗口期

### 3. 自动化交易
- 基于异常信号的自动下单
- 风险管理和仓位控制
- 止损/止盈自动执行
- 实时 PnL 追踪
- 交易统计和性能分析

### 4. 多渠道通知
- **Telegram**：机器人推送，支持群组
- **微信**：企业微信通知
- **Email**：SMTP 邮件告警
- **SMS**：短信通知（支持 Twilio/阿里云）

### 5. REST API & WebSocket
- RESTful API 用于查询和配置
- WebSocket 实时数据推送
- 支持多客户端并发连接
- 订阅式数据分发

### 6. 分布式事件处理
- Fluvio 作为事件总线
- 高吞吐量消息处理
- 事件持久化和回放
- 水平扩展支持

## 快速开始

### 环境要求

- Docker 20.10+
- Docker Compose 2.0+
- Make (可选，用于简化命令)

或本地开发：
- Rust 1.70+
- PostgreSQL 14+
- Fluvio 0.21+

### Docker 快速部署（推荐）

1. **克隆仓库**
```bash
git clone https://github.com/your-repo/crypto-monitor.git
cd crypto-monitor
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件，配置您的 API 密钥和通知服务
```

3. **启动所有服务**
```bash
# 使用 Make（推荐）
make prod

# 或直接使用 docker-compose
docker-compose up -d
```

4. **检查服务健康状态**
```bash
make health
# 或
./scripts/healthcheck.sh
```

5. **访问服务**
- API: http://localhost:8080
- WebSocket: ws://localhost:8081/ws
- Grafana: http://localhost:3000 (admin/admin)
- Prometheus: http://localhost:9090

### 开发环境

```bash
# 启动开发环境（带热重载）
make dev

# 查看日志
make logs

# 停止服务
make down
```

### 本地开发（不使用 Docker）

1. **安装依赖**
```bash
# 安装 Fluvio
curl -fsS https://hub.infinyon.cloud/install.sh | bash
fluvio cluster start

# 安装 sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres
```

2. **配置数据库**
```bash
# 创建数据库
createdb crypto_monitor

# 运行迁移
sqlx migrate run
```

3. **配置应用**
```bash
cp config.example.yaml config.yaml
# 编辑 config.yaml，填入您的配置信息
```

4. **运行应用**
```bash
# 开发模式
cargo run --bin crypto-monitor -- --config config.yaml

# 或使用 cargo-watch 热重载
cargo install cargo-watch
cargo watch -x 'run --bin crypto-monitor -- --config config.yaml'
```

## API 文档

### REST API 端点

#### 系统状态
- `GET /health` - 健康检查
- `GET /api/v1/status` - 系统状态

#### 市场数据
- `GET /api/v1/market/stats` - 市场统计
- `GET /api/v1/market/history` - 历史数据
- `GET /api/v1/market/orderbook` - 订单簿

#### 异常检测
- `GET /api/v1/anomalies` - 异常列表
- `GET /api/v1/anomalies/stats` - 异常统计

#### 交易管理
- `GET /api/v1/trading/config` - 交易配置
- `POST /api/v1/trading/config` - 更新配置
- `GET /api/v1/trading/positions` - 持仓列表
- `GET /api/v1/trading/orders` - 订单列表
- `POST /api/v1/trading/orders` - 下单
- `DELETE /api/v1/trading/orders/:id` - 撤单

#### 告警配置
- `GET /api/v1/alerts/config` - 告警配置
- `POST /api/v1/alerts/config` - 更新配置
- `GET /api/v1/alerts/history` - 告警历史

### WebSocket 订阅

连接到 `ws://localhost:8080/ws`

订阅消息格式：
```json
{
  "msg_type": "Subscribe",
  "data": {
    "channel": "anomalies",
    "symbols": ["BTC/USDT", "ETH/USDT"],
    "exchanges": ["binance", "coinbase"]
  }
}
```

## 配置说明

### 交易所配置
```yaml
exchanges:
  - name: binance
    enabled: true
    symbols: ["BTC/USDT", "ETH/USDT"]
    subscriptions: ["trades", "orderbook"]
```

### 异常检测配置
```yaml
anomaly_detection:
  volume_threshold_multiplier: 3.0  # Z-score 阈值
  price_change_percentage: 5.0      # 价格变化百分比
  lookback_window_minutes: 60       # 历史窗口
  min_samples: 30                   # 最小样本数
```

### 自动交易配置
```yaml
trading:
  auto_trading_enabled: false
  max_position_size: 1000.0
  risk_percentage: 2.0
  stop_loss_percentage: 3.0
  take_profit_percentage: 6.0
```

## 性能优化

- 使用 Rust 实现高性能数据处理
- 基于 Tokio 的异步并发
- Zero-copy 消息传递
- 索引优化的 O(1) 查找
- 连接池和批处理

## 监控指标

系统提供以下监控指标：

- 数据延迟（延迟 < 100ms）
- 消息吞吐量（> 100k msg/s）
- 异常检测准确率
- 交易执行成功率
- API 响应时间

## 安全考虑

- API 密钥加密存储
- TLS/SSL 加密传输
- 访问控制和认证
- 审计日志
- 风险限制

## 开发路线图

- [ ] 支持更多交易所
- [ ] 机器学习异常检测
- [ ] 策略回测框架
- [ ] 移动端应用
- [ ] Kubernetes 部署
- [ ] 多语言 SDK

## Docker 使用指南

### 常用命令

```bash
# 构建镜像
make build          # 生产环境
make build-dev      # 开发环境

# 启动服务
make up             # 启动所有服务
make dev            # 开发模式（热重载）
make prod           # 生产模式

# 管理服务
make logs           # 查看日志
make health         # 健康检查
make down           # 停止服务
make clean          # 清理所有容器和卷

# 数据库操作
make migrate        # 运行迁移
make db-shell       # 进入数据库控制台
make db-backup      # 备份数据库
make db-restore FILE=backup.sql.gz  # 恢复数据库

# 监控
make monitor-start  # 启动 Grafana 和 Prometheus
make monitor-stop   # 停止监控服务

# Fluvio
make fluvio-topics  # 查看所有主题
make fluvio-consume TOPIC=crypto-monitor.market.trades  # 消费主题消息
```

### 生产部署

1. **SSL 证书配置**
```bash
# 将 SSL 证书放置在 nginx/ssl/ 目录
cp /path/to/cert.pem nginx/ssl/
cp /path/to/key.pem nginx/ssl/
```

2. **环境变量配置**
```bash
# 生产环境使用独立的 .env 文件
cp .env.example .env.prod
# 编辑 .env.prod，设置生产环境配置
```

3. **启动生产环境**
```bash
docker-compose --env-file .env.prod up -d
```

4. **扩展服务**
```bash
# 扩展应用实例数量
docker-compose up -d --scale crypto-monitor=3
```

### 故障排查

1. **查看服务状态**
```bash
docker-compose ps
docker-compose logs crypto-monitor
```

2. **进入容器调试**
```bash
docker-compose exec crypto-monitor bash
```

3. **检查网络连接**
```bash
docker-compose exec crypto-monitor ping postgres
docker-compose exec crypto-monitor nc -zv fluvio 9003
```

4. **查看资源使用**
```bash
docker stats
```

## 性能优化建议

### Docker 优化

1. **资源限制**
```yaml
# docker-compose.yml
services:
  crypto-monitor:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

2. **日志管理**
```yaml
# docker-compose.yml
services:
  crypto-monitor:
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

3. **健康检查优化**
```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

## 贡献指南

欢迎提交 Issue 和 Pull Request！

### 开发流程

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 进行代码检查
- 编写单元测试和集成测试
- 更新相关文档

## 许可证

MIT License

## 联系方式

- GitHub Issues: [项目地址]/issues
- Email: contact@example.com