# Crypto Monitor API 快速测试命令

## 1. 基础健康检查

```bash
# 健康检查
curl http://localhost:8080/health

# 系统状态
curl http://localhost:8080/api/v1/status | jq
```

## 2. 市场数据查询

```bash
# 获取所有市场统计
curl http://localhost:8080/api/v1/market/stats | jq

# 获取 BTC/USDT 市场统计
curl "http://localhost:8080/api/v1/market/stats?symbol=BTC/USDT" | jq

# 获取币安交易所数据
curl "http://localhost:8080/api/v1/market/stats?exchange=binance" | jq

# 获取历史数据（最近10条）
curl "http://localhost:8080/api/v1/market/history?symbol=BTC/USDT&limit=10" | jq

# 获取订单簿
curl "http://localhost:8080/api/v1/market/orderbook?symbol=BTC/USDT&exchange=binance" | jq
```

## 3. 异常检测

```bash
# 获取所有异常
curl http://localhost:8080/api/v1/anomalies | jq

# 获取交易量异常
curl "http://localhost:8080/api/v1/anomalies?anomaly_type=VolumeSpike" | jq

# 获取高严重性异常
curl "http://localhost:8080/api/v1/anomalies?severity=High" | jq

# 获取异常统计
curl http://localhost:8080/api/v1/anomalies/stats | jq
```

## 4. 交易配置

```bash
# 获取交易配置
curl http://localhost:8080/api/v1/trading/config | jq

# 启用自动交易
curl -X POST http://localhost:8080/api/v1/trading/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "max_position_size": 1000.0,
    "stop_loss_percentage": 3.0,
    "take_profit_percentage": 6.0
  }' | jq

# 禁用自动交易
curl -X POST http://localhost:8080/api/v1/trading/config \
  -H "Content-Type: application/json" \
  -d '{"enabled": false, "symbol": "BTC/USDT", "exchange": "binance", "max_position_size": 0, "stop_loss_percentage": 0, "take_profit_percentage": 0}' | jq
```

## 5. 持仓和订单

```bash
# 获取所有持仓
curl http://localhost:8080/api/v1/trading/positions | jq

# 获取开放持仓
curl "http://localhost:8080/api/v1/trading/positions?status=open" | jq

# 获取所有订单
curl http://localhost:8080/api/v1/trading/orders | jq

# 下市价单（买入）
curl -X POST http://localhost:8080/api/v1/trading/orders \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "side": "buy",
    "type": "market",
    "quantity": 0.001
  }' | jq

# 下限价单（卖出）
curl -X POST http://localhost:8080/api/v1/trading/orders \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "side": "sell",
    "type": "limit",
    "quantity": 0.001,
    "price": 50000.0
  }' | jq
```

## 6. 告警配置

```bash
# 获取告警配置
curl http://localhost:8080/api/v1/alerts/config | jq

# 配置 Telegram 告警
curl -X POST http://localhost:8080/api/v1/alerts/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "channels": [{
      "channel_type": "Telegram",
      "config": {
        "chat_id": "-123456789",
        "send_images": true
      }
    }],
    "severity_threshold": "Medium"
  }' | jq

# 获取告警历史
curl http://localhost:8080/api/v1/alerts/history | jq

# 获取严重告警
curl "http://localhost:8080/api/v1/alerts/history?severity=Critical&limit=10" | jq
```

## 7. WebSocket 测试

```bash
# 安装 wscat
npm install -g wscat

# 连接 WebSocket
wscat -c ws://localhost:8081/ws

# 连接后发送以下消息：

# 订阅所有事件
{"msg_type":"Subscribe","data":{"channel":"all","symbols":[],"exchanges":[]}}

# 订阅 BTC/USDT 市场数据
{"msg_type":"Subscribe","data":{"channel":"market","symbols":["BTC/USDT"],"exchanges":["binance"]}}

# 订阅异常事件
{"msg_type":"Subscribe","data":{"channel":"anomalies","symbols":[],"exchanges":[]}}

# 订阅告警
{"msg_type":"Subscribe","data":{"channel":"alerts","symbols":[],"exchanges":[]}}

# 发送心跳
{"msg_type":"Heartbeat","data":{}}

# 取消订阅
{"msg_type":"Unsubscribe","data":{"channel":"market","symbols":["BTC/USDT"],"exchanges":["binance"]}}
```

## 8. 批量操作

```bash
# 批量下单
curl -X POST http://localhost:8080/api/v1/trading/orders/batch \
  -H "Content-Type: application/json" \
  -d '{
    "orders": [
      {
        "symbol": "BTC/USDT",
        "exchange": "binance",
        "side": "buy",
        "type": "limit",
        "quantity": 0.001,
        "price": 40000.0
      },
      {
        "symbol": "ETH/USDT",
        "exchange": "binance",
        "side": "buy",
        "type": "limit",
        "quantity": 0.01,
        "price": 2500.0
      }
    ]
  }' | jq
```

## 9. 复杂查询示例

```bash
# 时间范围查询
curl "http://localhost:8080/api/v1/market/history?symbol=BTC/USDT&exchange=binance&from=2024-01-01T00:00:00Z&to=2024-01-01T01:00:00Z&limit=100" | jq

# 多条件异常查询
curl "http://localhost:8080/api/v1/anomalies?symbol=BTC/USDT&exchange=binance&anomaly_type=PriceSpike&severity=High&from=2024-01-01T00:00:00Z&limit=50" | jq

# 分页查询
curl "http://localhost:8080/api/v1/market/history?symbol=BTC/USDT&page=1&per_page=20" | jq
```

## 10. 性能测试

```bash
# 使用 ab (Apache Bench) 进行压力测试
ab -n 1000 -c 10 http://localhost:8080/health

# 使用 wrk 进行压力测试
wrk -t4 -c100 -d30s http://localhost:8080/api/v1/market/stats

# 使用 curl 测试响应时间
curl -w "@curl-format.txt" -o /dev/null -s http://localhost:8080/api/v1/market/stats
```

创建 `curl-format.txt` 文件：
```
time_namelookup:  %{time_namelookup}s\n
time_connect:  %{time_connect}s\n
time_appconnect:  %{time_appconnect}s\n
time_pretransfer:  %{time_pretransfer}s\n
time_redirect:  %{time_redirect}s\n
time_starttransfer:  %{time_starttransfer}s\n
----------\n
time_total:  %{time_total}s\n
```

## 11. 监控指标

```bash
# 获取 Prometheus 指标
curl http://localhost:8080/metrics

# 获取特定指标
curl -s http://localhost:8080/metrics | grep http_requests_total

# 获取应用性能指标
curl -s http://localhost:8080/metrics | grep -E "response_time|request_count|error_rate"
```

## 12. 错误测试

```bash
# 测试 404
curl -i http://localhost:8080/api/v1/nonexistent

# 测试无效请求体
curl -X POST http://localhost:8080/api/v1/trading/orders \
  -H "Content-Type: application/json" \
  -d '{"invalid": "data"}' | jq

# 测试方法不允许
curl -X DELETE http://localhost:8080/api/v1/market/stats -i
```

## 快速测试脚本

将所有基本测试放在一个脚本中：

```bash
#!/bin/bash

API_BASE="http://localhost:8080"

echo "Testing Health..."
curl -s $API_BASE/health | jq '.'

echo -e "\nTesting System Status..."
curl -s $API_BASE/api/v1/status | jq '.'

echo -e "\nTesting Market Stats..."
curl -s $API_BASE/api/v1/market/stats | jq '.'

echo -e "\nTesting Anomalies..."
curl -s $API_BASE/api/v1/anomalies | jq '.'

echo -e "\nTesting Trading Config..."
curl -s $API_BASE/api/v1/trading/config | jq '.'

echo -e "\nAll basic tests completed!"
```

保存为 `quick-test.sh` 并运行：
```bash
chmod +x quick-test.sh
./quick-test.sh
```